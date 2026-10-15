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
    let connection = {
        let store = state.connections.lock().await;
        store.create(input).await?
    };
    if connection.tunnel.is_some() {
        state.tunnels.lock().await.start(connection.clone()).await?;
    }
    Ok(connection)
}

#[tauri::command]
pub async fn update_connection(
    state: State<'_, AppState>,
    id: String,
    input: ConnectionInput,
) -> AppResult<Connection> {
    let connection = {
        let store = state.connections.lock().await;
        store.update(&id, input).await?
    };
    state.tunnels.lock().await.stop(&id).await;
    if connection.tunnel.is_some() {
        state.tunnels.lock().await.start(connection.clone()).await?;
    }
    Ok(connection)
}

#[tauri::command]
pub async fn delete_connection(
    state: State<'_, AppState>,
    id: String,
) -> AppResult<()> {
    state.tunnels.lock().await.stop(&id).await;
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

/// Test the values currently entered in the connection form without saving
/// metadata or credentials to the connection store or OS keychain.
#[tauri::command]
pub async fn test_connection_input(
    state: State<'_, AppState>,
    id: Option<String>,
    input: ConnectionInput,
) -> AppResult<String> {
    super::log_err(async {
        // In the edit form, an empty token means "keep the saved token". Use
        // that credential for the test without writing it back to the keychain.
        let token = if input.token.is_empty() {
            match id.as_deref() {
                Some(id) => crate::connections::read_token(id).await?,
                None => String::new(),
            }
        } else {
            input.token
        };
        let tunnel = input.tunnel.clone();
        let endpoint = tunnel
            .as_ref()
            .map(crate::connections::TunnelConfig::local_endpoint)
            .unwrap_or(input.endpoint);
        let use_existing_tunnel = if let (Some(id), Some(config)) = (id.as_ref(), tunnel.as_ref()) {
            let existing = state.connections.lock().await.get(id)?;
            if existing.tunnel.as_ref() == Some(config) {
                state.tunnels.lock().await.ensure_started(&existing).await?;
                true
            } else {
                false
            }
        } else {
            false
        };
        let mut temporary_tunnel = match tunnel.as_ref() {
            Some(config) if !use_existing_tunnel => Some(crate::tunnels::start_temporary(config).await?),
            _ => None,
        };
        let client = crate::spicedb::SpiceDbClient::connect(
            crate::spicedb::client::DialConfig {
                endpoint,
                insecure: tunnel.is_some() || input.insecure,
                token,
            },
        )
        .await?;
        let result = client.ping().await;
        if let Some(child) = temporary_tunnel.as_mut() {
            let _ = child.kill().await;
        }
        result
    }
    .await)
}
