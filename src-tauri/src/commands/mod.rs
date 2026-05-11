pub mod connections;
pub mod schema;

use crate::error::AppResult;
use crate::spicedb::{client::DialConfig, SpiceDbClient};
use crate::AppState;
use tauri::State;

/// Build a SpiceDB client for a known connection id, pulling the token out of
/// the keychain. Centralized here so every command uses the same path.
pub(crate) async fn client_for(
    state: &State<'_, AppState>,
    connection_id: &str,
) -> AppResult<SpiceDbClient> {
    let store = state.connections.lock().await;
    let conn = store.get(connection_id)?;
    let token = store.token(connection_id)?;
    drop(store);

    SpiceDbClient::connect(DialConfig {
        endpoint: conn.endpoint,
        insecure: conn.insecure,
        token,
    })
    .await
}
