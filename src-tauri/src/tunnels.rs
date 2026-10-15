//! Managed private tunnels for cloud-hosted SpiceDB services.

use crate::connections::{Connection, TunnelConfig};
use crate::error::{AppError, AppResult};
use chrono::{DateTime, Utc};
use serde::Serialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::AsyncReadExt;
use tokio::net::TcpStream;
use tokio::process::{Child, Command};
use tokio::sync::{watch, Mutex};

#[derive(Clone, Debug, Serialize)]
pub struct TunnelStatus {
    pub connection_id: String,
    pub state: String,
    pub local_endpoint: String,
    pub message: Option<String>,
    pub changed_at: DateTime<Utc>,
}

struct TunnelHandle {
    cancel: watch::Sender<bool>,
    status: Arc<Mutex<TunnelStatus>>,
    task: tauri::async_runtime::JoinHandle<()>,
}

#[derive(Default)]
pub struct TunnelManager {
    handles: HashMap<String, TunnelHandle>,
}

impl TunnelManager {
    pub async fn start(&mut self, connection: Connection) -> AppResult<()> {
        self.stop(&connection.id).await;
        let Some(config) = connection.tunnel.clone() else {
            return Ok(());
        };
        let (cancel, cancel_rx) = watch::channel(false);
        let status = Arc::new(Mutex::new(TunnelStatus {
            connection_id: connection.id.clone(),
            state: "connecting".into(),
            local_endpoint: config.local_endpoint(),
            message: None,
            changed_at: Utc::now(),
        }));
        let task = tauri::async_runtime::spawn(run_managed_tunnel(config, status.clone(), cancel_rx));
        self.handles.insert(
            connection.id.clone(),
            TunnelHandle {
                cancel,
                status,
                task,
            },
        );
        Ok(())
    }

    pub async fn stop(&mut self, connection_id: &str) {
        if let Some(mut handle) = self.handles.remove(connection_id) {
            let _ = handle.cancel.send(true);
            if tokio::time::timeout(Duration::from_secs(2), &mut handle.task)
                .await
                .is_err()
            {
                handle.task.abort();
            }
        }
    }

    pub async fn stop_all(&mut self) {
        let handles = std::mem::take(&mut self.handles);
        for mut handle in handles.into_values() {
            let _ = handle.cancel.send(true);
            if tokio::time::timeout(Duration::from_secs(2), &mut handle.task)
                .await
                .is_err()
            {
                handle.task.abort();
            }
        }
    }

    pub async fn statuses(&self) -> Vec<TunnelStatus> {
        let mut rows = Vec::with_capacity(self.handles.len());
        for handle in self.handles.values() {
            rows.push(handle.status.lock().await.clone());
        }
        rows.sort_by(|a, b| a.connection_id.cmp(&b.connection_id));
        rows
    }

    pub async fn ensure_started(&mut self, connection: &Connection) -> AppResult<()> {
        if connection.tunnel.is_none() {
            return Ok(());
        }
        if !self.handles.contains_key(&connection.id) {
            self.start(connection.clone()).await?;
        }
        let status = self
            .handles
            .get(&connection.id)
            .map(|handle| handle.status.clone())
            .ok_or_else(|| AppError::Other("Tunnel did not start".into()))?;
        wait_until_connected(status, Duration::from_secs(20)).await
    }
}

async fn run_managed_tunnel(
    config: TunnelConfig,
    status: Arc<Mutex<TunnelStatus>>,
    mut cancel: watch::Receiver<bool>,
) {
    let mut retry_seconds = 1u64;
    loop {
        if *cancel.borrow() {
            set_status(&status, "stopped", None).await;
            return;
        }
        set_status(&status, "connecting", None).await;
        match spawn_railway_tunnel(&config).await {
            Ok(mut child) => {
                match wait_for_port(&config, &mut child, Duration::from_secs(15)).await {
                    Ok(()) => {
                        retry_seconds = 1;
                        set_status(&status, "connected", None).await;
                        tokio::select! {
                            result = child.wait() => {
                                let message = match result {
                                    Ok(exit) => format!("Railway tunnel exited with {exit}"),
                                    Err(error) => format!("Railway tunnel stopped: {error}"),
                                };
                                set_status(&status, "reconnecting", Some(message)).await;
                            }
                            _ = cancel.changed() => {
                                let _ = child.kill().await;
                                set_status(&status, "stopped", None).await;
                                return;
                            }
                        }
                    }
                    Err(error) => {
                        let _ = child.kill().await;
                        set_status(&status, "reconnecting", Some(error.to_string())).await;
                    }
                }
            }
            Err(error) => {
                set_status(&status, "reconnecting", Some(error.to_string())).await;
            }
        }
        tokio::select! {
            _ = tokio::time::sleep(Duration::from_secs(retry_seconds)) => {}
            _ = cancel.changed() => {
                set_status(&status, "stopped", None).await;
                return;
            }
        }
        retry_seconds = (retry_seconds * 2).min(30);
    }
}

pub async fn start_temporary(config: &TunnelConfig) -> AppResult<Child> {
    let mut child = spawn_railway_tunnel(config).await?;
    if let Err(error) = wait_for_port(config, &mut child, Duration::from_secs(20)).await {
        let _ = child.kill().await;
        return Err(error);
    }
    Ok(child)
}

async fn spawn_railway_tunnel(config: &TunnelConfig) -> AppResult<Child> {
    let TunnelConfig::Railway {
        project,
        environment,
        service,
        remote_port,
        local_port,
    } = config;
    let railway = find_executable("railway", &["/opt/homebrew/bin/railway", "/usr/local/bin/railway"])
        .ok_or_else(|| AppError::Other("Railway CLI was not found. Install it and run `railway login` first.".into()))?;
    let output = tokio::time::timeout(Duration::from_secs(15), Command::new(railway)
        .args([
            "ssh", "config", "--dry-run", "--project", project, "--environment",
            environment, "--service", service,
        ])
        .output())
        .await
        .map_err(|_| AppError::Other("Railway CLI timed out. Check your network and run `railway login` in Terminal.".into()))?
        .map_err(|error| AppError::Other(format!("Could not run Railway CLI: {error}")))?;
    if !output.status.success() {
        let message = String::from_utf8_lossy(&output.stderr);
        return Err(AppError::Other(safe_railway_error(&message)));
    }
    let config_text = String::from_utf8_lossy(&output.stdout);
    let host = directive(&config_text, "HostName")
        .ok_or_else(|| AppError::Other("Railway SSH config did not include HostName".into()))?;
    let user = directive(&config_text, "User")
        .ok_or_else(|| AppError::Other("Railway SSH config did not include User".into()))?;
    let ssh = find_executable("ssh", &["/usr/bin/ssh"])
        .ok_or_else(|| AppError::Other("OpenSSH was not found on this computer".into()))?;
    let forwarding = format!("127.0.0.1:{local_port}:127.0.0.1:{remote_port}");
    let control_path = dirs::data_dir()
        .ok_or_else(|| AppError::Other("Could not locate the application data directory".into()))?
        .join("dev.spicelens.app")
        .join(format!("railway-tunnel-{local_port}.sock"));
    if control_path.exists() {
        let _ = Command::new(&ssh)
            .args(["-S", &control_path.to_string_lossy(), "-O", "exit", &format!("{user}@{host}")])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .await;
        let _ = std::fs::remove_file(&control_path);
    }
    if std::net::TcpListener::bind(("127.0.0.1", *local_port)).is_err() {
        return Err(AppError::Other(format!(
            "Local port {local_port} is already in use. Choose another local port."
        )));
    }
    Command::new(ssh)
        .args([
            "-N",
            "-M",
            "-S", &control_path.to_string_lossy(),
            "-o", "ExitOnForwardFailure=yes",
            "-o", "BatchMode=yes",
            "-o", "ForwardAgent=no",
            "-o", "ForwardX11=no",
            "-o", "ControlPersist=no",
            "-o", "ConnectTimeout=10",
            "-o", "StrictHostKeyChecking=accept-new",
            "-o", "ServerAliveInterval=30",
            "-o", "ServerAliveCountMax=3",
            "-L", &forwarding,
            &format!("{user}@{host}"),
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .map_err(|error| AppError::Other(format!("Could not start SSH: {error}")))
}

async fn wait_for_port(
    config: &TunnelConfig,
    child: &mut Child,
    timeout: Duration,
) -> AppResult<()> {
    let endpoint = config.local_endpoint();
    let deadline = tokio::time::Instant::now() + timeout;
    loop {
        if let Some(exit) = child
            .try_wait()
            .map_err(|error| AppError::Other(format!("Could not inspect SSH: {error}")))?
        {
            let mut stderr = String::new();
            if let Some(mut pipe) = child.stderr.take() {
                let _ = pipe.read_to_string(&mut stderr).await;
            }
            let detail = safe_ssh_error(stderr.trim(), &endpoint);
            return Err(AppError::Other(if detail.is_empty() {
                format!("Railway SSH tunnel exited with {exit}")
            } else {
                detail
            }));
        }
        if TcpStream::connect(&endpoint).await.is_ok() {
            return Ok(());
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(AppError::Other(format!(
                "Timed out waiting for the Railway tunnel on {endpoint}"
            )));
        }
        tokio::time::sleep(Duration::from_millis(150)).await;
    }
}

async fn wait_until_connected(
    status: Arc<Mutex<TunnelStatus>>,
    timeout: Duration,
) -> AppResult<()> {
    let deadline = tokio::time::Instant::now() + timeout;
    loop {
        let current = status.lock().await.clone();
        if current.state == "connected" {
            return Ok(());
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(AppError::Other(current.message.unwrap_or_else(|| {
                format!("Timed out waiting for tunnel on {}", current.local_endpoint)
            })));
        }
        tokio::time::sleep(Duration::from_millis(150)).await;
    }
}

async fn set_status(status: &Arc<Mutex<TunnelStatus>>, state: &str, message: Option<String>) {
    let mut status = status.lock().await;
    status.state = state.into();
    status.message = message.map(|value| value.chars().take(1000).collect());
    status.changed_at = Utc::now();
}

fn directive(config: &str, name: &str) -> Option<String> {
    config.lines().find_map(|line| {
        let mut parts = line.split_whitespace();
        (parts.next()? == name).then(|| parts.next().unwrap_or_default().to_string())
    })
}

fn find_executable(name: &str, fallbacks: &[&str]) -> Option<PathBuf> {
    if let Some(path) = std::env::var_os("PATH") {
        for directory in std::env::split_paths(&path) {
            let candidate = directory.join(name);
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    if let Some(home) = dirs::home_dir() {
        for relative in [".railway/bin", ".local/bin", "bin"] {
            let candidate = home.join(relative).join(name);
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    fallbacks
        .iter()
        .map(Path::new)
        .find(|path| path.is_file())
        .map(Path::to_path_buf)
}

fn safe_railway_error(stderr: &str) -> String {
    let normalized = stderr.to_ascii_lowercase();
    if normalized.contains("unauthorized")
        || normalized.contains("not logged")
        || normalized.contains("railway login")
    {
        "Railway authentication failed. Open Terminal and run `railway login`, then try again.".into()
    } else if normalized.contains("not found") {
        "Railway could not find that project, environment, or service. Check the Railway IDs and names.".into()
    } else {
        "Railway could not prepare the private tunnel. Check your network, Railway CLI login, and connection details.".into()
    }
}

fn safe_ssh_error(stderr: &str, endpoint: &str) -> String {
    let normalized = stderr.to_ascii_lowercase();
    if normalized.contains("address already in use") {
        format!("Local endpoint {endpoint} is already in use. Choose another local port.")
    } else if normalized.contains("permission denied") {
        "Railway rejected the SSH key. Register a key with `railway ssh keys` and try again.".into()
    } else if normalized.contains("host identification has changed") {
        "Railway's SSH host key changed. Review the warning in Terminal before reconnecting.".into()
    } else if normalized.contains("host key verification failed") {
        "Railway SSH host verification failed. Connect once from Terminal and review the host key.".into()
    } else if normalized.contains("connection timed out") || normalized.contains("operation timed out") {
        "The Railway SSH connection timed out. Check your network and try again.".into()
    } else if stderr.is_empty() {
        String::new()
    } else {
        "Railway SSH could not establish the private tunnel. Check your Railway login and registered SSH key.".into()
    }
}

/// Close any OpenSSH masters owned by SpiceDepot. This is intentionally
/// synchronous so application teardown cannot outrun the cleanup request.
pub fn shutdown_control_tunnels() {
    let Some(data_dir) = dirs::data_dir() else {
        return;
    };
    let app_dir = data_dir.join("dev.spicelens.app");
    let Ok(entries) = std::fs::read_dir(&app_dir) else {
        return;
    };
    let Some(ssh) = find_executable("ssh", &["/usr/bin/ssh"]) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let is_control_socket = path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.starts_with("railway-tunnel-") && name.ends_with(".sock"));
        if !is_control_socket {
            continue;
        }
        let _ = std::process::Command::new(&ssh)
            .args(["-S", &path.to_string_lossy(), "-O", "exit", "spicedepot-tunnel"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
        let _ = std::fs::remove_file(path);
    }
}
