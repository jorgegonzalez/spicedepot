pub mod connections;
pub mod lookup;
pub mod permissions;
pub mod relationships;
pub mod schema;

use crate::error::AppResult;
use crate::spicedb::{client::DialConfig, SpiceDbClient};
use crate::AppState;
use tauri::State;

/// Build a SpiceDB client for a known connection id, pulling the token out of
/// the keychain. Centralized here so every command uses the same path.
///
/// Lock discipline: we read the (fast) JSON-backed connection record under the
/// store mutex, then **drop the lock** before doing the keychain read or the
/// gRPC dial. Both of those can block for noticeable time (keychain auth
/// prompts on macOS; network round-trips on dial), and we don't want to
/// serialize unrelated commands behind them.
pub(crate) async fn client_for(
    state: &State<'_, AppState>,
    connection_id: &str,
) -> AppResult<SpiceDbClient> {
    let conn = {
        let store = state.connections.lock().await;
        store.get(connection_id)?
    };
    let token = crate::connections::read_token(connection_id).await?;

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
