use super::client_for;
use crate::error::AppResult;
use crate::spicedb::proto::{
    CheckPermissionRequest, Consistency, ConsistencyRequirement, ObjectReference,
    Permissionship as ProtoPermissionship, SubjectReference,
};
use crate::AppState;
use serde::{Deserialize, Serialize};
use tauri::State;

/// Frontend input for a single permission check.
///
/// `subject_relation` is for "subject relations" like `group:engineering#member` —
/// when the subject is itself referenced by a relation, not an atomic object.
/// Most checks leave it empty.
#[derive(Debug, Deserialize)]
pub struct CheckPermissionInput {
    pub resource_type: String,
    pub resource_id: String,
    pub permission: String,
    pub subject_type: String,
    pub subject_id: String,
    pub subject_relation: Option<String>,
}

/// Frontend-friendly Permissionship. We avoid sending the proto enum's
/// `PERMISSIONSHIP_*` names to the UI — they're noisy.
///
/// Shared with `commands::lookup` — both `CheckPermission` and the lookup
/// RPCs return the same `Permissionship` enum on the wire.
#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Permissionship {
    HasPermission,
    NoPermission,
    /// Permission depends on a caveat that needs more context to evaluate.
    ConditionalPermission,
    Unspecified,
}

impl From<ProtoPermissionship> for Permissionship {
    fn from(p: ProtoPermissionship) -> Self {
        match p {
            ProtoPermissionship::HasPermission => Self::HasPermission,
            ProtoPermissionship::NoPermission => Self::NoPermission,
            ProtoPermissionship::ConditionalPermission => Self::ConditionalPermission,
            ProtoPermissionship::Unspecified => Self::Unspecified,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct CheckPermissionOutput {
    pub permissionship: Permissionship,
    /// Opaque ZedToken (`checked_at.token`) — useful for "check at this revision"
    /// follow-ups once we expose Consistency in the UI.
    pub checked_at: Option<String>,
    /// When permissionship is `conditional_permission`, the names of context
    /// fields the caveat needs that weren't provided. Empty otherwise.
    pub missing_context: Vec<String>,
}

#[tauri::command]
pub async fn check_permission(
    state: State<'_, AppState>,
    connection_id: String,
    input: CheckPermissionInput,
) -> AppResult<CheckPermissionOutput> {
    super::log_err(async {
        let client = client_for(&state, &connection_id).await?;
        let req = CheckPermissionRequest {
            // Fully consistent: always read at the latest revision. The SpiceDB
            // default (MinimizeLatency) can return a stale snapshot, which is
            // surprising in an interactive testing tool — the user just wrote
            // a relationship and expects the next check to reflect it.
            consistency: Some(Consistency {
                requirement: Some(ConsistencyRequirement::FullyConsistent(true)),
            }),
            resource: Some(ObjectReference {
                object_type: input.resource_type,
                object_id: input.resource_id,
            }),
            permission: input.permission,
            subject: Some(SubjectReference {
                object: Some(ObjectReference {
                    object_type: input.subject_type,
                    object_id: input.subject_id,
                }),
                optional_relation: input.subject_relation.unwrap_or_default(),
            }),
            context: None,
            with_tracing: false,
        };
        let resp = client.check_permission(req).await?;

        let permissionship: Permissionship = resp.permissionship().into();
        let checked_at = resp.checked_at.map(|t| t.token);
        let missing_context = resp
            .partial_caveat_info
            .map(|p| p.missing_required_context)
            .unwrap_or_default();

        Ok(CheckPermissionOutput {
            permissionship,
            checked_at,
            missing_context,
        })
    }
    .await)
}
