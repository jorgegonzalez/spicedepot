use serde::Serialize;
use std::path::PathBuf;
use std::process::Command;

#[derive(Serialize)]
pub struct McpConnectResult {
    pub client: String,
    pub message: String,
}

#[tauri::command]
pub fn connect_mcp_client(client: String) -> Result<McpConnectResult, String> {
    let cli_name = match client.as_str() {
        "claude" => "claude",
        "codex" => "codex",
        _ => return Err("Choose Claude Code or Codex.".into()),
    };
    let cli = find_cli(cli_name).ok_or_else(|| format!(
        "Could not find the {cli_name} command line app. Install it and restart SpiceDepot, then try again."
    ))?;
    let executable = std::env::current_exe()
        .map_err(|e| format!("Could not locate the SpiceDepot executable: {e}"))?;

    let output = if client == "claude" {
        Command::new(cli)
            .args([
                "mcp",
                "add",
                "--transport",
                "stdio",
                "--scope",
                "user",
                "spicedepot",
                "--",
            ])
            .arg(executable)
            .arg("mcp")
            .output()
    } else {
        Command::new(cli)
            .args(["mcp", "add", "spicedepot", "--"])
            .arg(executable)
            .arg("mcp")
            .output()
    }
    .map_err(|e| format!("Could not configure {cli_name}: {e}"))?;

    if !output.status.success() {
        let details = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let details = if details.is_empty() {
            String::from_utf8_lossy(&output.stdout).trim().to_string()
        } else {
            details
        };
        return Err(if details.is_empty() {
            format!("{cli_name} could not add SpiceDepot (exit status {}). If a server named 'spicedepot' already exists, remove or rename it in {cli_name} and try again.", output.status)
        } else {
            format!("{cli_name} could not add SpiceDepot: {details}")
        });
    }
    Ok(McpConnectResult {
        client: client.clone(),
        message: format!("SpiceDepot is connected to {cli_name}. Restart {cli_name} if it does not appear right away."),
    })
}

fn find_cli(name: &str) -> Option<PathBuf> {
    let mut dirs: Vec<PathBuf> = std::env::var_os("PATH")
        .map(|path| std::env::split_paths(&path).collect())
        .unwrap_or_default();
    if let Some(home) = dirs::home_dir() {
        dirs.extend([
            home.join(".local/bin"),
            home.join(".cargo/bin"),
            home.join(".npm-global/bin"),
            home.join(".volta/bin"),
            home.join(".bun/bin"),
        ]);
    }
    #[cfg(target_os = "macos")]
    dirs.extend([
        PathBuf::from("/opt/homebrew/bin"),
        PathBuf::from("/usr/local/bin"),
    ]);
    #[cfg(target_os = "linux")]
    dirs.extend([PathBuf::from("/usr/local/bin"), PathBuf::from("/usr/bin")]);

    #[cfg(windows)]
    let candidates = [
        format!("{name}.exe"),
        format!("{name}.cmd"),
        name.to_owned(),
    ];
    #[cfg(not(windows))]
    let candidates = [name.to_owned()];
    dirs.into_iter()
        .flat_map(|dir| candidates.iter().map(move |candidate| dir.join(candidate)))
        .find(|path| path.is_file())
}
