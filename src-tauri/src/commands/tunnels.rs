use crate::tunnels::TunnelStatus;
use crate::AppState;
use tauri::State;

#[tauri::command]
pub async fn list_tunnel_statuses(
    state: State<'_, AppState>,
) -> Result<Vec<TunnelStatus>, String> {
    Ok(state.tunnels.lock().await.statuses().await)
}

#[tauri::command]
pub async fn start_tunnel(state: State<'_, AppState>, connection_id: String) -> Result<(), String> {
    let connection = state
        .connections
        .lock()
        .await
        .get(&connection_id)
        .map_err(|error| error.to_string())?;
    state
        .tunnels
        .lock()
        .await
        .start(connection)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn stop_tunnel(
    state: State<'_, AppState>,
    connection_id: String,
) -> Result<(), String> {
    state.tunnels.lock().await.stop(&connection_id).await;
    Ok(())
}
