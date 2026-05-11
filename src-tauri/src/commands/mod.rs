pub mod connections;
pub mod schema;

use crate::error::AppResult;
use crate::spicedb::{client::DialConfig, SpiceDbClient};
use crate::AppState;
use tauri::State;

/// Build a SpiceDB client for a known connection id, pulling the token out of
/// the keychain. Centralized here so every command uses the same path. The
/// store lock is released before the (potentially slow) gRPC dial so other
/// commands aren't starved.
pub(crate) async fn client_for(
    state: &State<'_, AppState>,
    connection_id: &str,
) -> AppResult<SpiceDbClient> {
    let store = state.connections.lock().await;
    let conn = store.get(connection_id)?;
    let token = store.token(connection_id).await?;
    drop(store);

    SpiceDbClient::connect(DialConfig {
        endpoint: conn.endpoint,
        insecure: conn.insecure,
        token,
    })
    .await
}

/// Log any `Err` returned by a command so it shows up in the Rust logs.
/// `AppError` only travels to the frontend as a single string; this is the
/// one place the full source chain (via `Display` of the underlying error)
/// gets recorded server-side.
pub(crate) fn log_err<T>(r: AppResult<T>) -> AppResult<T> {
    if let Err(ref e) = r {
        tracing::error!(error = %e, "command failed");
    }
    r
}
