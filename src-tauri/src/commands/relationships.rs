//! Relationship browser commands: read with optional filter + single-row
//! create/touch/delete via `WriteRelationships`.
//!
//! Bulk delete-by-filter (`PermissionsService.DeleteRelationships`) is out of
//! scope for v1 — when we add it, route it through a separate command so the
//! "are you sure?" UX can live on that path alone.

use super::client_for;
use crate::error::AppResult;
use crate::spicedb::proto::{
    Consistency, ConsistencyRequirement, DeleteRelationshipsRequest, ObjectReference,
    ReadRelationshipsRequest, Relationship, RelationshipFilter, RelationshipOperation,
    RelationshipUpdate, SubjectFilter, SubjectRelationFilter, SubjectReference,
    WriteRelationshipsRequest,
};
use crate::AppState;
use serde::{Deserialize, Serialize};
use tauri::State;

/// What the UI sends for both filtering reads and identifying a single row to
/// write or delete. Empty strings → "don't filter on this field".
#[derive(Debug, Clone, Deserialize)]
pub struct RelationshipFilterInput {
    pub resource_type: String,
    pub resource_id: Option<String>,
    pub relation: Option<String>,
    pub subject_type: Option<String>,
    pub subject_id: Option<String>,
    pub subject_relation: Option<String>,
}

/// One concrete relationship — used as the payload for create/touch/delete.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RelationshipInput {
    pub resource_type: String,
    pub resource_id: String,
    pub relation: String,
    pub subject_type: String,
    pub subject_id: String,
    pub subject_relation: Option<String>,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WriteOperation {
    /// CREATE: errors if the relationship already exists.
    Create,
    /// TOUCH: upsert — creates if missing, succeeds silently if it already
    /// matches. Recommended default for interactive use.
    Touch,
    /// DELETE: removes the relationship if present, no-op if missing.
    Delete,
}

impl From<WriteOperation> for RelationshipOperation {
    fn from(op: WriteOperation) -> Self {
        match op {
            WriteOperation::Create => RelationshipOperation::Create,
            WriteOperation::Touch => RelationshipOperation::Touch,
            WriteOperation::Delete => RelationshipOperation::Delete,
        }
    }
}

/// Flat shape we send to the frontend — easier to render in a table than the
/// nested proto type.
#[derive(Debug, Clone, Serialize)]
pub struct RelationshipRow {
    pub resource_type: String,
    pub resource_id: String,
    pub relation: String,
    pub subject_type: String,
    pub subject_id: String,
    pub subject_relation: Option<String>,
    /// Caveat name if the relationship was written with one. We don't yet
    /// expose the caveat context shape to the UI.
    pub caveat_name: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ReadRelationshipsOutput {
    pub items: Vec<RelationshipRow>,
    pub read_at: Option<String>,
}

#[tauri::command]
pub async fn read_relationships(
    state: State<'_, AppState>,
    connection_id: String,
    filter: RelationshipFilterInput,
    limit: Option<u32>,
) -> AppResult<ReadRelationshipsOutput> {
    super::log_err(async {
        let client = client_for(&state, &connection_id).await?;
        let req = ReadRelationshipsRequest {
            consistency: Some(Consistency {
                requirement: Some(ConsistencyRequirement::FullyConsistent(true)),
            }),
            relationship_filter: Some(build_relationship_filter(&filter)),
            optional_limit: limit.unwrap_or(0),
            optional_cursor: None,
        };

        let responses = client.read_relationships(req).await?;
        let read_at = responses
            .first()
            .and_then(|r| r.read_at.as_ref().map(|t| t.token.clone()));
        let items = responses
            .into_iter()
            .filter_map(|r| r.relationship.map(relationship_to_row))
            .collect();
        Ok(ReadRelationshipsOutput { items, read_at })
    }
    .await)
}

#[tauri::command]
pub async fn write_relationship(
    state: State<'_, AppState>,
    connection_id: String,
    operation: WriteOperation,
    relationship: RelationshipInput,
) -> AppResult<()> {
    super::log_err(async {
        let client = client_for(&state, &connection_id).await?;
        let proto_op: RelationshipOperation = operation.into();
        let update = RelationshipUpdate {
            operation: proto_op as i32,
            relationship: Some(input_to_relationship(relationship)),
        };
        client
            .write_relationships(WriteRelationshipsRequest {
                updates: vec![update],
                optional_preconditions: Vec::new(),
                optional_transaction_metadata: None,
            })
            .await?;
        Ok(())
    }
    .await)
}

/// SpiceDB caps relationship updates per WriteRelationships call. The
/// default server config is `MaximumUpdatesPerWrite: 1000`; we use 500 to
/// stay safely under that and to keep individual gRPC payloads small.
const BULK_WRITE_CHUNK: usize = 500;

#[derive(Debug, serde::Serialize)]
pub struct BulkWriteOutput {
    pub written_count: u32,
    pub chunks: u32,
}

/// Bulk-write a batch of relationships under a single operation kind
/// (create / touch / delete). Used by the CSV import flow.
///
/// Chunks the request server-side so the user doesn't need to think about
/// SpiceDB's per-write limit. A 50,000-row import becomes 100 gRPC calls.
#[tauri::command]
pub async fn bulk_write_relationships(
    state: State<'_, AppState>,
    connection_id: String,
    operation: WriteOperation,
    relationships: Vec<RelationshipInput>,
) -> AppResult<BulkWriteOutput> {
    super::log_err(async {
        if relationships.is_empty() {
            return Ok(BulkWriteOutput { written_count: 0, chunks: 0 });
        }
        let client = client_for(&state, &connection_id).await?;
        let proto_op: RelationshipOperation = operation.into();
        let total = relationships.len() as u32;
        let mut chunks = 0u32;

        for chunk in relationships.chunks(BULK_WRITE_CHUNK) {
            let updates: Vec<RelationshipUpdate> = chunk
                .iter()
                .cloned()
                .map(|r| RelationshipUpdate {
                    operation: proto_op as i32,
                    relationship: Some(input_to_relationship(r)),
                })
                .collect();
            client
                .write_relationships(WriteRelationshipsRequest {
                    updates,
                    optional_preconditions: Vec::new(),
                    optional_transaction_metadata: None,
                })
                .await?;
            chunks += 1;
        }
        Ok(BulkWriteOutput {
            written_count: total,
            chunks,
        })
    }
    .await)
}

/// Bulk delete every relationship matching the filter. Optional `limit` caps
/// the deletion size (without it, SpiceDB rejects calls whose match-set
/// exceeds the server's `MaxDeleteRelationshipsLimit`, default 1000).
///
/// Returns the deletion count. The frontend gets a single number it can
/// display in a "Deleted N relationships" toast.
#[derive(Debug, serde::Serialize)]
pub struct BulkDeleteOutput {
    pub deleted_count: u64,
    /// `complete` (all matching rows deleted) or `partial` (limit hit).
    pub progress: &'static str,
    pub deleted_at: Option<String>,
}

#[tauri::command]
pub async fn bulk_delete_relationships(
    state: tauri::State<'_, AppState>,
    connection_id: String,
    filter: RelationshipFilterInput,
    limit: Option<u32>,
) -> AppResult<BulkDeleteOutput> {
    super::log_err(async {
        let client = client_for(&state, &connection_id).await?;
        let limit_value = limit.unwrap_or(0);
        let resp = client
            .delete_relationships(DeleteRelationshipsRequest {
                relationship_filter: Some(build_relationship_filter(&filter)),
                optional_preconditions: Vec::new(),
                optional_limit: limit_value,
                // If the caller specified a limit, allow partial deletion so
                // the server respects the cap instead of erroring out.
                optional_allow_partial_deletions: limit_value > 0,
                optional_transaction_metadata: None,
            })
            .await?;
        use crate::spicedb::proto::authzed::api::v1::delete_relationships_response::DeletionProgress;
        let progress = match DeletionProgress::try_from(resp.deletion_progress)
            .unwrap_or(DeletionProgress::Unspecified)
        {
            DeletionProgress::Complete => "complete",
            DeletionProgress::Partial => "partial",
            DeletionProgress::Unspecified => "unspecified",
        };
        Ok(BulkDeleteOutput {
            deleted_count: resp.relationships_deleted_count,
            progress,
            deleted_at: resp.deleted_at.map(|t| t.token),
        })
    }
    .await)
}

// ---- conversions ----------------------------------------------------------

fn build_relationship_filter(f: &RelationshipFilterInput) -> RelationshipFilter {
    let subject_filter = if some_non_empty(&f.subject_type) {
        Some(SubjectFilter {
            subject_type: f.subject_type.clone().unwrap_or_default(),
            optional_subject_id: f.subject_id.clone().unwrap_or_default(),
            optional_relation: f.subject_relation.as_ref().filter(|s| !s.is_empty()).map(
                |r| SubjectRelationFilter {
                    relation: r.clone(),
                },
            ),
        })
    } else {
        None
    };
    RelationshipFilter {
        resource_type: f.resource_type.clone(),
        optional_resource_id: f.resource_id.clone().unwrap_or_default(),
        optional_relation: f.relation.clone().unwrap_or_default(),
        optional_resource_id_prefix: String::new(),
        optional_subject_filter: subject_filter,
    }
}

fn input_to_relationship(input: RelationshipInput) -> Relationship {
    Relationship {
        resource: Some(ObjectReference {
            object_type: input.resource_type,
            object_id: input.resource_id,
        }),
        relation: input.relation,
        subject: Some(SubjectReference {
            object: Some(ObjectReference {
                object_type: input.subject_type,
                object_id: input.subject_id,
            }),
            optional_relation: input.subject_relation.unwrap_or_default(),
        }),
        optional_caveat: None,
        optional_expires_at: None,
    }
}

fn relationship_to_row(r: Relationship) -> RelationshipRow {
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

fn some_non_empty(s: &Option<String>) -> bool {
    s.as_deref().is_some_and(|v| !v.is_empty())
}
