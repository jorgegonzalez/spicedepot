//! tonic-generated proto code for the authzed v1 API.
//!
//! `tonic_build` writes one `.rs` per package into `OUT_DIR`. Cross-package
//! references (e.g. `authzed.api.v1` referencing `google.rpc.Status`) use
//! relative `super::…` paths that escape one level past the package, so the
//! sibling packages must be mounted at the same module level. We mount both
//! `authzed` and `google` inside this module, which makes the generated paths
//! resolve correctly. `google.protobuf.*` types come from `prost_types` and
//! don't need to be mounted.

#[allow(clippy::all)]
pub mod authzed {
    pub mod api {
        pub mod v1 {
            tonic::include_proto!("authzed.api.v1");
        }
    }
}

#[allow(clippy::all)]
pub mod google {
    pub mod rpc {
        tonic::include_proto!("google.rpc");
    }
}

// Convenience re-exports for the parts we use elsewhere.
pub use authzed::api::v1::{
    check_permission_response::Permissionship,
    consistency::Requirement as ConsistencyRequirement,
    permissions_service_client::PermissionsServiceClient,
    relationship_update::Operation as RelationshipOperation,
    schema_service_client::SchemaServiceClient,
    subject_filter::RelationFilter as SubjectRelationFilter,
    watch_service_client::WatchServiceClient,
    CheckPermissionRequest, CheckPermissionResponse, Consistency, DeleteRelationshipsRequest,
    DeleteRelationshipsResponse, LookupResourcesRequest, LookupResourcesResponse,
    LookupSubjectsRequest, LookupSubjectsResponse, ObjectReference,
    ReadRelationshipsRequest, ReadRelationshipsResponse, ReadSchemaRequest,
    ReadSchemaResponse, Relationship, RelationshipFilter, RelationshipUpdate,
    SubjectFilter, SubjectReference, WatchRequest, WatchResponse,
    WriteRelationshipsRequest, WriteRelationshipsResponse, WriteSchemaRequest,
};
