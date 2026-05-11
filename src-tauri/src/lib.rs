//! SpiceLens Tauri application library.
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
mod spicedb;

use std::sync::Arc;
use tauri::Manager;
use tokio::sync::Mutex;

/// App-wide state, mounted on the Tauri builder via `.manage()`.
pub struct AppState {
    pub connections: Arc<Mutex<connections::ConnectionStore>>,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "spicelens_lib=info,warn".into()),
        )
        .init();

    tauri::Builder::default()
        .plugin(tauri_plugin_store::Builder::default().build())
        .setup(|app| {
            let store = connections::ConnectionStore::new(app.handle().clone())?;
            app.manage(AppState {
                connections: Arc::new(Mutex::new(store)),
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::connections::list_connections,
            commands::connections::get_connection,
            commands::connections::create_connection,
            commands::connections::update_connection,
            commands::connections::delete_connection,
            commands::connections::test_connection,
            commands::schema::read_schema,
            commands::schema::write_schema,
            commands::permissions::check_permission,
            commands::lookup::lookup_resources,
            commands::lookup::lookup_subjects,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
