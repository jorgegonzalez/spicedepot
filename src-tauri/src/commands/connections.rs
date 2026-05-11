use super::client_for;
use crate::connections::{Connection, ConnectionInput};
use crate::error::AppResult;
use crate::AppState;
use tauri::State;

#[tauri::command]
pub async fn list_connections(state: State<'_, AppState>) -> AppResult<Vec<Connection>> {
    state.connections.lock().await.list()
}

#[tauri::command]
pub async fn get_connection(
    state: State<'_, AppState>,
    id: String,
) -> AppResult<Connection> {
    state.connections.lock().await.get(&id)
}

#[tauri::command]
pub async fn create_connection(
    state: State<'_, AppState>,
    input: ConnectionInput,
) -> AppResult<Connection> {
    state.connections.lock().await.create(input)
}

#[tauri::command]
pub async fn update_connection(
    state: State<'_, AppState>,
    id: String,
    input: ConnectionInput,
) -> AppResult<Connection> {
    state.connections.lock().await.update(&id, input)
}

#[tauri::command]
pub async fn delete_connection(
    state: State<'_, AppState>,
    id: String,
) -> AppResult<()> {
    state.connections.lock().await.delete(&id)
}

/// Dial the connection and return a friendly status string.
/// Used by the "Test" button in the connection manager.
#[tauri::command]
pub async fn test_connection(
    state: State<'_, AppState>,
    id: String,
) -> AppResult<String> {
    let client = client_for(&state, &id).await?;
    client.ping().await
}
