use crate::mcp_activity::McpActivityEntry;
use crate::AppState;
use tauri::State;

#[tauri::command]
pub async fn list_mcp_activity(
    state: State<'_, AppState>,
) -> Result<Vec<McpActivityEntry>, String> {
    state
        .mcp_activity
        .lock()
        .await
        .list()
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn clear_mcp_activity(state: State<'_, AppState>) -> Result<(), String> {
    state
        .mcp_activity
        .lock()
        .await
        .clear()
        .map_err(|error| error.to_string())
}
