//! SpiceDepot Tauri application library.
//!
//! Architecture (high level):
//! - `spicedb::proto` — generated tonic + prost code for the authzed gRPC API.
//! - `spicedb::client` — connection-aware wrapper that builds a `tonic::transport::Channel`
//!   (plaintext for `insecure`, rustls for TLS) and exposes typed service stubs.
//! - `connections` — persistent store for `Connection` records. Metadata lives in
//!   tauri-plugin-store (JSON file), tokens in the OS keychain via `keyring`.
//! - `commands` — Tauri command handlers; the only thing the frontend talks to.
//! - `error::AppError` — single error type that serializes to a friendly string for
//!   the frontend, while preserving the underlying source for logging.

mod commands;
mod connections;
mod error;
mod mcp;
mod mcp_activity;
mod spicedb;
mod tunnels;

use std::collections::HashMap;
use std::sync::Arc;
use tauri::Manager;
use tokio::sync::Mutex;
use tokio::task::AbortHandle;

/// App-wide state, mounted on the Tauri builder via `.manage()`.
pub struct AppState {
    pub connections: Arc<Mutex<connections::ConnectionStore>>,
    pub mcp_activity: Arc<Mutex<mcp_activity::McpActivityStore>>,
    pub tunnels: Arc<Mutex<tunnels::TunnelManager>>,
    /// Active Watch streams keyed by an opaque watch_id. Aborting the handle
    /// drops the tonic stream which cancels the gRPC call.
    pub watches: Arc<Mutex<HashMap<String, AbortHandle>>>,
}

/// Run the stdio bridge that an MCP client launches on behalf of the app.
/// Tool execution and keychain access stay in the desktop process.
pub fn run_mcp() -> anyhow::Result<()> {
    mcp::run_stdio()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "spicedepot_lib=info,warn".into()),
        )
        .init();

    tauri::Builder::default()
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let store = connections::ConnectionStore::new(app.handle().clone())?;
            let connections = Arc::new(Mutex::new(store));
            let data_dir = app.path().app_data_dir()?;
            let mcp_activity = Arc::new(Mutex::new(mcp_activity::McpActivityStore::new(
                app.handle().clone(),
            )?));
            let tunnels = Arc::new(Mutex::new(tunnels::TunnelManager::default()));
            mcp::start_server(
                connections.clone(),
                mcp_activity.clone(),
                tunnels.clone(),
                &data_dir,
            )?;
            app.manage(AppState {
                connections: connections.clone(),
                mcp_activity,
                tunnels: tunnels.clone(),
                watches: Arc::new(Mutex::new(HashMap::new())),
            });
            tauri::async_runtime::spawn(async move {
                let saved = connections.lock().await.list().unwrap_or_default();
                for connection in saved.into_iter().filter(|item| item.tunnel.is_some()) {
                    let _ = tunnels.lock().await.start(connection).await;
                }
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::mcp::connect_mcp_client,
            commands::mcp_activity::list_mcp_activity,
            commands::mcp_activity::clear_mcp_activity,
            commands::tunnels::list_tunnel_statuses,
            commands::tunnels::start_tunnel,
            commands::tunnels::stop_tunnel,
            commands::connections::list_connections,
            commands::connections::get_connection,
            commands::connections::create_connection,
            commands::connections::update_connection,
            commands::connections::delete_connection,
            commands::connections::test_connection,
            commands::connections::test_connection_input,
            commands::schema::read_schema,
            commands::schema::write_schema,
            commands::permissions::check_permission,
            commands::lookup::lookup_resources,
            commands::lookup::lookup_subjects,
            commands::relationships::read_relationships,
            commands::relationships::write_relationship,
            commands::relationships::bulk_write_relationships,
            commands::relationships::bulk_delete_relationships,
            commands::watch::watch_start,
            commands::watch::watch_stop,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app, event| {
            if matches!(event, tauri::RunEvent::ExitRequested { .. } | tauri::RunEvent::Exit) {
                let tunnels = app.state::<AppState>().tunnels.clone();
                tauri::async_runtime::block_on(async move {
                    tunnels.lock().await.stop_all().await;
                });
                tunnels::shutdown_control_tunnels();
            }
        });
}
