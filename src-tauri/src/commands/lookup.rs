//! `LookupResources` and `LookupSubjects` commands.
//!
//! Both are server-streaming gRPC RPCs; we drain the stream into a Vec inside
//! `SpiceDbClient` (see its doc-comment for the rationale). Server-side
//! SpiceDB caps response counts (default 1000); the optional `limit` field
//! here lets the UI request fewer if it wants. We don't surface pagination
//! cursors yet — when we add a "load more" button, both this layer and the
//! frontend will need cursor plumbing.

use super::client_for;
use super::permissions::Permissionship;
use crate::error::AppResult;
use crate::spicedb::proto::{
    Consistency, ConsistencyRequirement, LookupResourcesRequest, LookupSubjectsRequest,
    ObjectReference, Permissionship as ProtoPermissionship, SubjectReference,
};
use crate::AppState;
use serde::{Deserialize, Serialize};
use tauri::State;

fn fully_consistent() -> Consistency {
    Consistency {
        requirement: Some(ConsistencyRequirement::FullyConsistent(true)),
    }
}

// ---- LookupResources ------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct LookupResourcesInput {
    pub resource_type: String,
    pub permission: String,
    pub subject_type: String,
    pub subject_id: String,
    pub subject_relation: Option<String>,
    /// Optional client-side cap. `None` (or 0) means "use SpiceDB's default".
    pub limit: Option<u32>,
}

#[derive(Debug, Serialize)]
pub struct LookupResourceItem {
    pub resource_id: String,
    pub permissionship: Permissionship,
    pub missing_context: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct LookupResourcesOutput {
    pub items: Vec<LookupResourceItem>,
    pub looked_up_at: Option<String>,
}

#[tauri::command]
pub async fn lookup_resources(
    state: State<'_, AppState>,
    connection_id: String,
    input: LookupResourcesInput,
) -> AppResult<LookupResourcesOutput> {
    super::log_err(async {
        let client = client_for(&state, &connection_id).await?;
        let req = LookupResourcesRequest {
            consistency: Some(fully_consistent()),
            resource_object_type: input.resource_type,
            permission: input.permission,
            subject: Some(SubjectReference {
                object: Some(ObjectReference {
                    object_type: input.subject_type,
                    object_id: input.subject_id,
                }),
                optional_relation: input.subject_relation.unwrap_or_default(),
            }),
            context: None,
            optional_limit: input.limit.unwrap_or(0),
            optional_cursor: None,
            with_debug: false,
        };

        let responses = client.lookup_resources(req).await?;
        let looked_up_at = responses
            .first()
            .and_then(|r| r.looked_up_at.as_ref().map(|t| t.token.clone()));
        let items = responses
            .into_iter()
            .map(|r| LookupResourceItem {
                resource_id: r.resource_object_id,
                permissionship: ProtoPermissionship::try_from(r.permissionship)
                    .unwrap_or(ProtoPermissionship::Unspecified)
                    .into(),
                missing_context: r
                    .partial_caveat_info
                    .map(|p| p.missing_required_context)
                    .unwrap_or_default(),
            })
            .collect();
        Ok(LookupResourcesOutput {
            items,
            looked_up_at,
        })
    }
    .await)
}

// ---- LookupSubjects -------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct LookupSubjectsInput {
    pub resource_type: String,
    pub resource_id: String,
    pub permission: String,
    pub subject_type: String,
    pub subject_relation: Option<String>,
    pub limit: Option<u32>,
}

#[derive(Debug, Serialize)]
pub struct LookupSubjectItem {
    pub subject_id: String,
    pub permissionship: Permissionship,
    /// When `subject_id == "*"` (wildcard), these IDs are explicitly excluded
    /// from the wildcard. Empty for non-wildcard subjects.
    pub excluded_subject_ids: Vec<String>,
    pub missing_context: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct LookupSubjectsOutput {
    pub items: Vec<LookupSubjectItem>,
    pub looked_up_at: Option<String>,
}

#[tauri::command]
pub async fn lookup_subjects(
    state: State<'_, AppState>,
    connection_id: String,
    input: LookupSubjectsInput,
) -> AppResult<LookupSubjectsOutput> {
    super::log_err(async {
        let client = client_for(&state, &connection_id).await?;
        let req = LookupSubjectsRequest {
            consistency: Some(fully_consistent()),
            resource: Some(ObjectReference {
                object_type: input.resource_type,
                object_id: input.resource_id,
            }),
            permission: input.permission,
            subject_object_type: input.subject_type,
            optional_subject_relation: input.subject_relation.unwrap_or_default(),
            context: None,
            optional_concrete_limit: input.limit.unwrap_or(0),
            optional_cursor: None,
            wildcard_option: 0, // WILDCARD_OPTION_UNSPECIFIED — server default
        };

        let responses = client.lookup_subjects(req).await?;
        let looked_up_at = responses
            .first()
            .and_then(|r| r.looked_up_at.as_ref().map(|t| t.token.clone()));
        let items = responses
            .into_iter()
            .filter_map(|r| {
                let s = r.subject?;
                Some(LookupSubjectItem {
                    subject_id: s.subject_object_id,
                    permissionship: ProtoPermissionship::try_from(s.permissionship)
                        .unwrap_or(ProtoPermissionship::Unspecified)
                        .into(),
                    excluded_subject_ids: r
                        .excluded_subjects
                        .into_iter()
                        .map(|e| e.subject_object_id)
                        .collect(),
                    missing_context: s
                        .partial_caveat_info
                        .map(|p| p.missing_required_context)
                        .unwrap_or_default(),
                })
            })
            .collect();
        Ok(LookupSubjectsOutput {
            items,
            looked_up_at,
        })
    }
    .await)
}
