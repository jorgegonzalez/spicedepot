use super::client_for;
use crate::error::AppResult;
use crate::AppState;
use chrono::Utc;
use serde::Serialize;
use tauri::State;

#[derive(Serialize)]
pub struct SchemaResult {
    pub schema_text: String,
    pub read_at: Option<String>,
}

#[tauri::command]
pub async fn read_schema(
    state: State<'_, AppState>,
    connection_id: String,
) -> AppResult<SchemaResult> {
    super::log_err(async {
        let client = client_for(&state, &connection_id).await?;
        match client.read_schema().await {
            Ok(resp) => Ok(SchemaResult {
                schema_text: resp.schema_text,
                read_at: Some(Utc::now().to_rfc3339()),
            }),
            // No schema written yet — surface as empty string rather than an error.
            Err(crate::error::AppError::Grpc {
                code: tonic::Code::NotFound,
                ..
            }) => Ok(SchemaResult {
                schema_text: String::new(),
                read_at: Some(Utc::now().to_rfc3339()),
            }),
            Err(e) => Err(e),
        }
    }
    .await)
}

#[tauri::command]
pub async fn write_schema(
    state: State<'_, AppState>,
    connection_id: String,
    schema: String,
) -> AppResult<()> {
    super::log_err(async {
        let client = client_for(&state, &connection_id).await?;
        client.write_schema(schema).await
    }
    .await)
}
