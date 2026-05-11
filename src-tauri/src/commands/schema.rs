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
    let client = client_for(&state, &connection_id).await?;
    match client.read_schema().await {
        Ok(resp) => Ok(SchemaResult {
            schema_text: resp.schema_text,
            read_at: Some(Utc::now().to_rfc3339()),
        }),
        Err(crate::error::AppError::Grpc { code, .. })
            if code == tonic::Code::NotFound =>
        {
            // No schema written yet — surface as empty string rather than an error.
            Ok(SchemaResult {
                schema_text: String::new(),
                read_at: Some(Utc::now().to_rfc3339()),
            })
        }
        Err(e) => Err(e),
    }
}

#[tauri::command]
pub async fn write_schema(
    state: State<'_, AppState>,
    connection_id: String,
    schema: String,
) -> AppResult<()> {
    let client = client_for(&state, &connection_id).await?;
    client.write_schema(schema).await
}
