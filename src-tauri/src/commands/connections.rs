use super::client_for;
use crate::connections::{Connection, ConnectionInput};
use crate::error::AppResult;
use crate::AppState;
use tauri::State;

#[tauri::command]
pub async fn list_connections(state: State<'_, AppState>) -> AppResult<Vec<Connection>> {
    super::log_err(state.connections.lock().await.list())
}

#[tauri::command]
pub async fn get_connection(
    state: State<'_, AppState>,
    id: String,
) -> AppResult<Connection> {
    super::log_err(state.connections.lock().await.get(&id))
}

#[tauri::command]
pub async fn create_connection(
    state: State<'_, AppState>,
    input: ConnectionInput,
) -> AppResult<Connection> {
    let store = state.connections.lock().await;
    super::log_err(store.create(input).await)
}

#[tauri::command]
pub async fn update_connection(
    state: State<'_, AppState>,
    id: String,
    input: ConnectionInput,
) -> AppResult<Connection> {
    let store = state.connections.lock().await;
    super::log_err(store.update(&id, input).await)
}

#[tauri::command]
pub async fn delete_connection(
    state: State<'_, AppState>,
    id: String,
) -> AppResult<()> {
    let store = state.connections.lock().await;
    super::log_err(store.delete(&id).await)
}

/// Dial the connection and return a friendly status string.
/// Used by the "Test" button in the connection manager.
#[tauri::command]
pub async fn test_connection(
    state: State<'_, AppState>,
    id: String,
) -> AppResult<String> {
    super::log_err(async {
        let client = client_for(&state, &id).await?;
        client.ping().await
    }
    .await)
}
