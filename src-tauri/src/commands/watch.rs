//! Live `WatchService.Watch` stream → frontend bridge.
//!
//! Lifecycle:
//!   - `watch_start` spawns a Tokio task that drives the gRPC stream and
//!     forwards each `WatchResponse` as one or more `WatchEvent`s to the
//!     frontend via a `tauri::ipc::Channel<WatchEvent>`. It registers the
//!     task's abort handle under a fresh UUID and returns that to the caller.
//!   - `watch_stop` looks the handle up and calls `.abort()`. Dropping the
//!     stream tears down the underlying gRPC call.
//!
//! The map entry stays until either `watch_stop` removes it or the task
//! finishes on its own (terminal error / clean end-of-stream). We don't
//! garbage-collect completed entries — at the scale this UI runs (1-2 active
//! watches per session), it's not worth the bookkeeping.

use super::client_for;
use super::relationships::{RelationshipRow, WriteOperation};
use crate::error::{AppError, AppResult};
use crate::spicedb::proto::{Relationship, RelationshipOperation, WatchRequest};
use crate::AppState;
use serde::{Deserialize, Serialize};
use tauri::ipc::Channel;
use tauri::State;
use tokio_stream::StreamExt;

#[derive(Debug, Deserialize)]
pub struct WatchStartInput {
    /// If set, only emit updates to these resource object types.
    /// Empty list = all object types.
    pub object_types: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum WatchEvent {
    /// One relationship change.
    Update {
        operation: WriteOperation,
        relationship: RelationshipRow,
        /// The `changes_through` ZedToken from the batch this update arrived in.
        at: Option<String>,
    },
    /// Schema was modified at this revision. Schema content not included —
    /// the frontend can refetch via `read_schema` if it cares.
    SchemaChanged { at: Option<String> },
    /// Stream ended cleanly (rare — Watch is intended to run forever).
    Ended,
    /// Stream errored. The task has already exited; the frontend should call
    /// `watch_start` again to resume if desired.
    Error { message: String },
}

#[tauri::command]
pub async fn watch_start(
    state: State<'_, AppState>,
    connection_id: String,
    input: WatchStartInput,
    on_event: Channel<WatchEvent>,
) -> AppResult<String> {
    let client = client_for(&state, &connection_id).await?;
    let stream = client
        .watch_stream(WatchRequest {
            optional_object_types: input.object_types,
            optional_start_cursor: None,
            optional_relationship_filters: Vec::new(),
            optional_update_kinds: Vec::new(),
        })
        .await?;

    let watch_id = uuid::Uuid::new_v4().to_string();
    let watch_id_for_task = watch_id.clone();
    let watches = state.watches.clone();

    let handle = tokio::spawn(async move {
        let mut stream = stream;
        loop {
            match stream.next().await {
                Some(Ok(resp)) => {
                    let at = resp.changes_through.as_ref().map(|t| t.token.clone());
                    if resp.schema_updated {
                        let _ = on_event.send(WatchEvent::SchemaChanged { at: at.clone() });
                    }
                    for update in resp.updates {
                        if let Some(rel) = update.relationship {
                            let op = match RelationshipOperation::try_from(update.operation)
                                .unwrap_or(RelationshipOperation::Unspecified)
                            {
                                RelationshipOperation::Create => WriteOperation::Create,
                                RelationshipOperation::Touch => WriteOperation::Touch,
                                RelationshipOperation::Delete => WriteOperation::Delete,
                                RelationshipOperation::Unspecified => continue,
                            };
                            let _ = on_event.send(WatchEvent::Update {
                                operation: op,
                                relationship: flatten(rel),
                                at: at.clone(),
                            });
                        }
                    }
                }
                Some(Err(status)) => {
                    let _ = on_event.send(WatchEvent::Error {
                        message: format!("grpc {:?}: {}", status.code(), status.message()),
                    });
                    break;
                }
                None => {
                    let _ = on_event.send(WatchEvent::Ended);
                    break;
                }
            }
        }
        // Self-cleanup so the map doesn't accumulate completed handles.
        let mut map = watches.lock().await;
        map.remove(&watch_id_for_task);
    });

    state
        .watches
        .lock()
        .await
        .insert(watch_id.clone(), handle.abort_handle());
    Ok(watch_id)
}

#[tauri::command]
pub async fn watch_stop(
    state: State<'_, AppState>,
    watch_id: String,
) -> AppResult<()> {
    let handle = state.watches.lock().await.remove(&watch_id);
    match handle {
        Some(h) => {
            h.abort();
            Ok(())
        }
        None => Err(AppError::NotFound(watch_id)),
    }
}

fn flatten(r: Relationship) -> RelationshipRow {
    let resource = r.resource.unwrap_or_default();
    let subject = r.subject.unwrap_or_default();
    let subject_obj = subject.object.unwrap_or_default();
    RelationshipRow {
        resource_type: resource.object_type,
        resource_id: resource.object_id,
        relation: r.relation,
        subject_type: subject_obj.object_type,
        subject_id: subject_obj.object_id,
        subject_relation: Some(subject.optional_relation).filter(|s| !s.is_empty()),
        caveat_name: r.optional_caveat.map(|c| c.caveat_name),
    }
}
