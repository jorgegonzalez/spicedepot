//! Connection-aware SpiceDB client.
//!
//! Each call constructs a fresh tonic `Channel` per connection. We don't pool
//! channels yet — feature usage is interactive, latency matters less than
//! correctness. When the relationship browser / lookup arrive and start
//! streaming, swap in a pooled `Arc<Channel>` per connection id.
//!
//! The token is attached as an `authorization: Bearer <token>` metadata header
//! on every request, matching the SpiceDB pre-shared-key convention.

use crate::error::{AppError, AppResult};
use crate::spicedb::proto::{
    CheckPermissionRequest, CheckPermissionResponse, LookupResourcesRequest,
    LookupResourcesResponse, LookupSubjectsRequest, LookupSubjectsResponse,
    PermissionsServiceClient, ReadRelationshipsRequest, ReadRelationshipsResponse,
    ReadSchemaRequest, ReadSchemaResponse, SchemaServiceClient, WriteRelationshipsRequest,
    WriteRelationshipsResponse, WriteSchemaRequest,
};
use http::Uri;
use tonic::metadata::MetadataValue;
use tonic::transport::{Channel, ClientTlsConfig, Endpoint};
use tonic::{Request, Status};

/// Settings needed to dial a SpiceDB server.
#[derive(Clone, Debug)]
pub struct DialConfig {
    pub endpoint: String,
    pub insecure: bool,
    pub token: String,
}

#[derive(Clone)]
pub struct SpiceDbClient {
    channel: Channel,
    bearer: MetadataValue<tonic::metadata::Ascii>,
}

impl SpiceDbClient {
    pub async fn connect(cfg: DialConfig) -> AppResult<Self> {
        let uri = build_uri(&cfg.endpoint, cfg.insecure)?;

        let mut endpoint = Endpoint::from(uri.clone())
            .connect_timeout(std::time::Duration::from_secs(8))
            .timeout(std::time::Duration::from_secs(30))
            // gRPC keepalive — helpful through NAT/load balancers.
            .keep_alive_while_idle(true)
            .tcp_keepalive(Some(std::time::Duration::from_secs(60)));

        if !cfg.insecure {
            // Use rustls with native roots. tls-roots feature on tonic
            // pulls in webpki-roots and rustls-native-certs.
            let tls = ClientTlsConfig::new().with_native_roots();
            endpoint = endpoint.tls_config(tls)?;
        }

        let channel = endpoint.connect().await?;

        let bearer = format!("Bearer {}", cfg.token)
            .parse::<MetadataValue<_>>()
            .map_err(|e| AppError::Other(format!("invalid token (non-ASCII?): {e}")))?;

        Ok(Self { channel, bearer })
    }

    fn auth<T>(&self, req: T) -> Request<T> {
        let mut r = Request::new(req);
        r.metadata_mut()
            .insert("authorization", self.bearer.clone());
        r
    }

    pub fn schema(&self) -> SchemaServiceClient<Channel> {
        SchemaServiceClient::new(self.channel.clone())
    }

    pub fn permissions(&self) -> PermissionsServiceClient<Channel> {
        PermissionsServiceClient::new(self.channel.clone())
    }

    pub async fn read_schema(&self) -> AppResult<ReadSchemaResponse> {
        let mut svc = self.schema();
        let resp = svc.read_schema(self.auth(ReadSchemaRequest {})).await?;
        Ok(resp.into_inner())
    }

    pub async fn write_schema(&self, schema: String) -> AppResult<()> {
        let mut svc = self.schema();
        svc.write_schema(self.auth(WriteSchemaRequest { schema }))
            .await?;
        Ok(())
    }

    pub async fn check_permission(
        &self,
        req: CheckPermissionRequest,
    ) -> AppResult<CheckPermissionResponse> {
        let mut svc = self.permissions();
        let resp = svc.check_permission(self.auth(req)).await?;
        Ok(resp.into_inner())
    }

    /// LookupResources is a server-streaming RPC. For an interactive GUI we
    /// drain the stream into a Vec rather than threading it through to the
    /// frontend — SpiceDB caps the response count server-side (default 1000),
    /// and the latency from waiting for the full response is tolerable for
    /// what the UI actually needs to render.
    ///
    /// If/when we add a "load more" button or want progressive rendering,
    /// swap this for a Tauri `Channel<T>` emitting each item as it arrives.
    pub async fn lookup_resources(
        &self,
        req: LookupResourcesRequest,
    ) -> AppResult<Vec<LookupResourcesResponse>> {
        let mut svc = self.permissions();
        let mut stream = svc.lookup_resources(self.auth(req)).await?.into_inner();
        let mut out = Vec::new();
        while let Some(msg) = stream.message().await? {
            out.push(msg);
        }
        Ok(out)
    }

    pub async fn lookup_subjects(
        &self,
        req: LookupSubjectsRequest,
    ) -> AppResult<Vec<LookupSubjectsResponse>> {
        let mut svc = self.permissions();
        let mut stream = svc.lookup_subjects(self.auth(req)).await?.into_inner();
        let mut out = Vec::new();
        while let Some(msg) = stream.message().await? {
            out.push(msg);
        }
        Ok(out)
    }

    /// Server-streaming. Same Vec-drain pattern as the lookups — see their
    /// doc-comment for the rationale and the swap-to-Channel path.
    pub async fn read_relationships(
        &self,
        req: ReadRelationshipsRequest,
    ) -> AppResult<Vec<ReadRelationshipsResponse>> {
        let mut svc = self.permissions();
        let mut stream = svc.read_relationships(self.auth(req)).await?.into_inner();
        let mut out = Vec::new();
        while let Some(msg) = stream.message().await? {
            out.push(msg);
        }
        Ok(out)
    }

    /// Unary. The frontend currently sends one update at a time; nothing here
    /// prevents batching when we want it.
    pub async fn write_relationships(
        &self,
        req: WriteRelationshipsRequest,
    ) -> AppResult<WriteRelationshipsResponse> {
        let mut svc = self.permissions();
        let resp = svc.write_relationships(self.auth(req)).await?;
        Ok(resp.into_inner())
    }

    /// Best-effort liveness check. We try `ReadSchema` and treat `NOT_FOUND`
    /// (no schema yet) and `OK` as both meaning "we successfully reached a
    /// SpiceDB". Any other gRPC code propagates as an error.
    pub async fn ping(&self) -> AppResult<String> {
        let mut svc = self.schema();
        match svc.read_schema(self.auth(ReadSchemaRequest {})).await {
            Ok(_) => Ok("Connected. Schema present.".into()),
            Err(s) if s.code() == tonic::Code::NotFound => {
                Ok("Connected. No schema written yet.".into())
            }
            Err(s) => Err(AppError::from(s)),
        }
    }
}

/// Normalize a user-entered endpoint into a tonic `Uri`.
///
/// Accepts:
///   - `host:port`                → http://host:port (insecure) / https://host:port (TLS)
///   - `http://host:port`         → only valid in insecure mode
///   - `https://host:port`        → only valid in TLS mode
///   - `grpc://` / `grpcs://`     → mapped to http/https
fn build_uri(endpoint: &str, insecure: bool) -> AppResult<Uri> {
    let trimmed = endpoint.trim();
    if trimmed.is_empty() {
        return Err(AppError::InvalidEndpoint("endpoint is empty".into()));
    }

    let normalized = if let Some(rest) = trimmed.strip_prefix("grpc://") {
        format!("http://{rest}")
    } else if let Some(rest) = trimmed.strip_prefix("grpcs://") {
        format!("https://{rest}")
    } else if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
        trimmed.to_string()
    } else if insecure {
        format!("http://{trimmed}")
    } else {
        format!("https://{trimmed}")
    };

    // Sanity-check scheme matches the insecure flag.
    let uri: Uri = normalized
        .parse()
        .map_err(|e: http::uri::InvalidUri| AppError::InvalidEndpoint(e.to_string()))?;
    let scheme = uri.scheme_str().unwrap_or("");
    match (insecure, scheme) {
        (true, "http") | (false, "https") => Ok(uri),
        (true, "https") => Err(AppError::InvalidEndpoint(
            "endpoint uses https but the connection is marked insecure".into(),
        )),
        (false, "http") => Err(AppError::InvalidEndpoint(
            "endpoint uses http but the connection is marked secure (enable Insecure to use plaintext)".into(),
        )),
        _ => Err(AppError::InvalidEndpoint(format!(
            "unsupported scheme {scheme:?}"
        ))),
    }
}

#[allow(dead_code)]
fn _coerce_status(s: Status) -> AppError {
    AppError::from(s)
}

#[cfg(test)]
mod tests {
    use super::{build_uri, DialConfig, SpiceDbClient};

    #[test]
    fn host_port_insecure_becomes_http() {
        let u = build_uri("localhost:50051", true).unwrap();
        assert_eq!(u.scheme_str(), Some("http"));
        assert_eq!(u.host(), Some("localhost"));
        assert_eq!(u.port_u16(), Some(50051));
    }

    #[test]
    fn host_port_secure_becomes_https() {
        let u = build_uri("grpc.authzed.com:443", false).unwrap();
        assert_eq!(u.scheme_str(), Some("https"));
    }

    #[test]
    fn grpc_scheme_mapped() {
        assert_eq!(
            build_uri("grpc://example.com:50051", true)
                .unwrap()
                .scheme_str(),
            Some("http")
        );
        assert_eq!(
            build_uri("grpcs://example.com:443", false)
                .unwrap()
                .scheme_str(),
            Some("https")
        );
    }

    #[test]
    fn scheme_flag_mismatch_errors() {
        assert!(build_uri("https://example.com:443", true).is_err());
        assert!(build_uri("http://example.com:50051", false).is_err());
    }

    /// End-to-end test against a live SpiceDB. Skipped unless
    /// `SPICEDB_TEST_ENDPOINT` (and `SPICEDB_TEST_TOKEN`) are set, so it
    /// doesn't run in plain `cargo test`.
    ///
    ///   SPICEDB_TEST_ENDPOINT=localhost:50051 \
    ///   SPICEDB_TEST_TOKEN=spicelens-test-key \
    ///   cargo test -- --ignored insecure_roundtrip
    #[tokio::test]
    #[ignore]
    async fn insecure_roundtrip() {
        let endpoint = std::env::var("SPICEDB_TEST_ENDPOINT")
            .expect("set SPICEDB_TEST_ENDPOINT, e.g. localhost:50051");
        let token = std::env::var("SPICEDB_TEST_TOKEN")
            .expect("set SPICEDB_TEST_TOKEN to the SpiceDB preshared key");

        let client = SpiceDbClient::connect(DialConfig {
            endpoint,
            insecure: true,
            token,
        })
        .await
        .expect("connect");

        // Empty schema should come back as NotFound, which our code surfaces as
        // a clean error from read_schema. The Tauri command handler turns that
        // into an empty string; here we just want to verify the wire works.
        let initial = client.read_schema().await;
        match &initial {
            Ok(_) => {} // server already had a schema (re-running test) — fine
            Err(crate::error::AppError::Grpc { code, .. })
                if *code == tonic::Code::NotFound => {}
            Err(e) => panic!("unexpected initial read_schema error: {e}"),
        }

        // Write a known schema, then read it back and verify equality.
        let schema = "definition user {}\n\
                      definition document {\n\
                          relation viewer: user\n\
                          permission view = viewer\n\
                      }\n";
        client
            .write_schema(schema.to_string())
            .await
            .expect("write_schema");

        let resp = client.read_schema().await.expect("read_schema after write");
        assert!(
            resp.schema_text.contains("definition document"),
            "schema text did not round-trip: {:?}",
            resp.schema_text
        );

        // Ping should now succeed with the "schema present" branch.
        let msg = client.ping().await.expect("ping");
        assert!(msg.contains("Schema present"), "ping returned: {msg}");
    }

    /// End-to-end permission check: write a schema, write a relationship via
    /// the low-level gRPC API, then verify `check_permission` returns
    /// HasPermission for the related subject and NoPermission for an
    /// unrelated one. Mirrors the production code path through
    /// `SpiceDbClient::check_permission`.
    #[tokio::test]
    #[ignore]
    async fn check_permission_with_relationship() {
        use crate::spicedb::proto::authzed::api::v1::{
            relationship_update::Operation as RelOp, ObjectReference, Relationship,
            RelationshipUpdate, SubjectReference, WriteRelationshipsRequest,
        };
        use crate::spicedb::proto::{CheckPermissionRequest, Permissionship};

        let endpoint = std::env::var("SPICEDB_TEST_ENDPOINT")
            .expect("set SPICEDB_TEST_ENDPOINT");
        let token = std::env::var("SPICEDB_TEST_TOKEN")
            .expect("set SPICEDB_TEST_TOKEN");

        let client = SpiceDbClient::connect(DialConfig {
            endpoint,
            insecure: true,
            token,
        })
        .await
        .expect("connect");

        // 1. Write a schema that defines a `view` permission backed by a
        //    `viewer` relation on `document`.
        client
            .write_schema(
                "definition user {}\n\
                 definition document {\n\
                     relation viewer: user\n\
                     permission view = viewer\n\
                 }\n"
                .into(),
            )
            .await
            .expect("write_schema");

        // 2. Write a single relationship: document:doc1#viewer -> user:alice.
        let mut perms = client.permissions();
        let relationship_to_alice = Relationship {
            resource: Some(ObjectReference {
                object_type: "document".into(),
                object_id: "doc1".into(),
            }),
            relation: "viewer".into(),
            subject: Some(SubjectReference {
                object: Some(ObjectReference {
                    object_type: "user".into(),
                    object_id: "alice".into(),
                }),
                optional_relation: String::new(),
            }),
            optional_caveat: None,
            optional_expires_at: None,
        };
        let mut req = tonic::Request::new(WriteRelationshipsRequest {
            updates: vec![RelationshipUpdate {
                operation: RelOp::Touch as i32,
                relationship: Some(relationship_to_alice),
            }],
            optional_preconditions: Vec::new(),
            optional_transaction_metadata: None,
        });
        req.metadata_mut()
            .insert("authorization", client.bearer.clone());
        perms.write_relationships(req).await.expect("write_relationships");

        // 3. alice should now have `view` on doc1.
        let allowed = client
            .check_permission(CheckPermissionRequest {
                consistency: Some(crate::spicedb::proto::Consistency {
                    requirement: Some(
                        crate::spicedb::proto::ConsistencyRequirement::FullyConsistent(
                            true,
                        ),
                    ),
                }),
                resource: Some(ObjectReference {
                    object_type: "document".into(),
                    object_id: "doc1".into(),
                }),
                permission: "view".into(),
                subject: Some(SubjectReference {
                    object: Some(ObjectReference {
                        object_type: "user".into(),
                        object_id: "alice".into(),
                    }),
                    optional_relation: String::new(),
                }),
                context: None,
                with_tracing: false,
            })
            .await
            .expect("check alice");
        assert_eq!(
            allowed.permissionship(),
            Permissionship::HasPermission,
            "alice should have view on doc1"
        );

        // 4. bob, who has no relationship, should be denied.
        let denied = client
            .check_permission(CheckPermissionRequest {
                consistency: Some(crate::spicedb::proto::Consistency {
                    requirement: Some(
                        crate::spicedb::proto::ConsistencyRequirement::FullyConsistent(
                            true,
                        ),
                    ),
                }),
                resource: Some(ObjectReference {
                    object_type: "document".into(),
                    object_id: "doc1".into(),
                }),
                permission: "view".into(),
                subject: Some(SubjectReference {
                    object: Some(ObjectReference {
                        object_type: "user".into(),
                        object_id: "bob".into(),
                    }),
                    optional_relation: String::new(),
                }),
                context: None,
                with_tracing: false,
            })
            .await
            .expect("check bob");
        assert_eq!(
            denied.permissionship(),
            Permissionship::NoPermission,
            "bob should be denied"
        );
    }

    /// End-to-end Lookup test: schema + several relationships, then verify
    /// `lookup_resources` enumerates the resources a subject can access and
    /// `lookup_subjects` enumerates the subjects on a given resource.
    #[tokio::test]
    #[ignore]
    async fn lookup_round_trip() {
        use crate::spicedb::proto::authzed::api::v1::{
            relationship_update::Operation as RelOp, ObjectReference, Relationship,
            RelationshipUpdate, SubjectReference, WriteRelationshipsRequest,
        };
        use crate::spicedb::proto::{
            Consistency, ConsistencyRequirement, LookupResourcesRequest,
            LookupSubjectsRequest,
        };

        let endpoint = std::env::var("SPICEDB_TEST_ENDPOINT")
            .expect("set SPICEDB_TEST_ENDPOINT");
        let token = std::env::var("SPICEDB_TEST_TOKEN")
            .expect("set SPICEDB_TEST_TOKEN");

        let client = SpiceDbClient::connect(DialConfig {
            endpoint,
            insecure: true,
            token,
        })
        .await
        .expect("connect");

        client
            .write_schema(
                "definition user {}\n\
                 definition document {\n\
                     relation viewer: user\n\
                     permission view = viewer\n\
                 }\n"
                .into(),
            )
            .await
            .expect("write_schema");

        // alice can view doc1 and doc2; bob can view doc1 only.
        let rels = [
            ("doc1", "alice"),
            ("doc2", "alice"),
            ("doc1", "bob"),
        ];
        let updates: Vec<RelationshipUpdate> = rels
            .iter()
            .map(|(doc, user)| RelationshipUpdate {
                operation: RelOp::Touch as i32,
                relationship: Some(Relationship {
                    resource: Some(ObjectReference {
                        object_type: "document".into(),
                        object_id: (*doc).into(),
                    }),
                    relation: "viewer".into(),
                    subject: Some(SubjectReference {
                        object: Some(ObjectReference {
                            object_type: "user".into(),
                            object_id: (*user).into(),
                        }),
                        optional_relation: String::new(),
                    }),
                    optional_caveat: None,
                    optional_expires_at: None,
                }),
            })
            .collect();
        let mut perms = client.permissions();
        let mut req = tonic::Request::new(WriteRelationshipsRequest {
            updates,
            optional_preconditions: Vec::new(),
            optional_transaction_metadata: None,
        });
        req.metadata_mut()
            .insert("authorization", client.bearer.clone());
        perms.write_relationships(req).await.expect("write_relationships");

        let consistency = Some(Consistency {
            requirement: Some(ConsistencyRequirement::FullyConsistent(true)),
        });

        // LookupResources: which docs can alice view? → doc1, doc2
        let resources_resp = client
            .lookup_resources(LookupResourcesRequest {
                consistency: consistency.clone(),
                resource_object_type: "document".into(),
                permission: "view".into(),
                subject: Some(SubjectReference {
                    object: Some(ObjectReference {
                        object_type: "user".into(),
                        object_id: "alice".into(),
                    }),
                    optional_relation: String::new(),
                }),
                context: None,
                optional_limit: 0,
                optional_cursor: None,
                with_debug: false,
            })
            .await
            .expect("lookup_resources");
        let mut got_resources: Vec<String> = resources_resp
            .iter()
            .map(|r| r.resource_object_id.clone())
            .collect();
        got_resources.sort();
        assert_eq!(
            got_resources,
            vec!["doc1".to_string(), "doc2".to_string()],
            "alice should be able to view exactly doc1 and doc2"
        );

        // LookupSubjects: who can view doc1? → alice, bob
        let subjects_resp = client
            .lookup_subjects(LookupSubjectsRequest {
                consistency,
                resource: Some(ObjectReference {
                    object_type: "document".into(),
                    object_id: "doc1".into(),
                }),
                permission: "view".into(),
                subject_object_type: "user".into(),
                optional_subject_relation: String::new(),
                context: None,
                optional_concrete_limit: 0,
                optional_cursor: None,
                wildcard_option: 0,
            })
            .await
            .expect("lookup_subjects");
        let mut got_subjects: Vec<String> = subjects_resp
            .iter()
            .filter_map(|r| r.subject.as_ref().map(|s| s.subject_object_id.clone()))
            .collect();
        got_subjects.sort();
        assert_eq!(
            got_subjects,
            vec!["alice".to_string(), "bob".to_string()],
            "doc1 should be viewable by alice and bob"
        );
    }

    /// End-to-end relationship browser test: write three relationships via
    /// `write_relationships`, read them back with various filters via
    /// `read_relationships`, delete one via a TOUCH→DELETE round-trip, then
    /// verify the read reflects the change.
    #[tokio::test]
    #[ignore]
    async fn relationships_round_trip() {
        use crate::spicedb::proto::authzed::api::v1::{
            relationship_update::Operation as RelOp, ObjectReference, Relationship,
            RelationshipFilter, RelationshipUpdate, SubjectFilter, SubjectReference,
            WriteRelationshipsRequest,
        };
        use crate::spicedb::proto::{
            Consistency, ConsistencyRequirement, ReadRelationshipsRequest,
        };

        let endpoint = std::env::var("SPICEDB_TEST_ENDPOINT")
            .expect("set SPICEDB_TEST_ENDPOINT");
        let token = std::env::var("SPICEDB_TEST_TOKEN")
            .expect("set SPICEDB_TEST_TOKEN");

        let client = SpiceDbClient::connect(DialConfig {
            endpoint,
            insecure: true,
            token,
        })
        .await
        .expect("connect");

        client
            .write_schema(
                "definition user {}\n\
                 definition document {\n\
                     relation viewer: user\n\
                 }\n"
                .into(),
            )
            .await
            .expect("write_schema");

        // Helper to build a Relationship value.
        let rel = |doc: &str, user: &str| Relationship {
            resource: Some(ObjectReference {
                object_type: "document".into(),
                object_id: doc.into(),
            }),
            relation: "viewer".into(),
            subject: Some(SubjectReference {
                object: Some(ObjectReference {
                    object_type: "user".into(),
                    object_id: user.into(),
                }),
                optional_relation: String::new(),
            }),
            optional_caveat: None,
            optional_expires_at: None,
        };

        // Write three relationships in one call.
        let updates = vec![
            RelationshipUpdate {
                operation: RelOp::Touch as i32,
                relationship: Some(rel("doc1", "alice")),
            },
            RelationshipUpdate {
                operation: RelOp::Touch as i32,
                relationship: Some(rel("doc2", "alice")),
            },
            RelationshipUpdate {
                operation: RelOp::Touch as i32,
                relationship: Some(rel("doc1", "bob")),
            },
        ];
        client
            .write_relationships(WriteRelationshipsRequest {
                updates,
                optional_preconditions: Vec::new(),
                optional_transaction_metadata: None,
            })
            .await
            .expect("write 3 relationships");

        let consistency = || {
            Some(Consistency {
                requirement: Some(ConsistencyRequirement::FullyConsistent(true)),
            })
        };

        // 1. All document relationships: 3 rows.
        let all = client
            .read_relationships(ReadRelationshipsRequest {
                consistency: consistency(),
                relationship_filter: Some(RelationshipFilter {
                    resource_type: "document".into(),
                    optional_resource_id: String::new(),
                    optional_relation: String::new(),
                    optional_resource_id_prefix: String::new(),
                    optional_subject_filter: None,
                }),
                optional_limit: 0,
                optional_cursor: None,
            })
            .await
            .expect("read all");
        assert_eq!(all.len(), 3, "expected 3 relationships, got {}", all.len());

        // 2. Filter on resource_id=doc1: 2 rows (alice + bob).
        let doc1 = client
            .read_relationships(ReadRelationshipsRequest {
                consistency: consistency(),
                relationship_filter: Some(RelationshipFilter {
                    resource_type: "document".into(),
                    optional_resource_id: "doc1".into(),
                    optional_relation: String::new(),
                    optional_resource_id_prefix: String::new(),
                    optional_subject_filter: None,
                }),
                optional_limit: 0,
                optional_cursor: None,
            })
            .await
            .expect("read doc1");
        assert_eq!(doc1.len(), 2, "expected 2 viewers on doc1");

        // 3. Filter on subject alice: 2 rows (doc1, doc2).
        let alice = client
            .read_relationships(ReadRelationshipsRequest {
                consistency: consistency(),
                relationship_filter: Some(RelationshipFilter {
                    resource_type: "document".into(),
                    optional_resource_id: String::new(),
                    optional_relation: String::new(),
                    optional_resource_id_prefix: String::new(),
                    optional_subject_filter: Some(SubjectFilter {
                        subject_type: "user".into(),
                        optional_subject_id: "alice".into(),
                        optional_relation: None,
                    }),
                }),
                optional_limit: 0,
                optional_cursor: None,
            })
            .await
            .expect("read alice");
        assert_eq!(alice.len(), 2, "expected alice to view 2 docs");

        // 4. Delete alice's viewer on doc2 via WriteRelationships(DELETE).
        client
            .write_relationships(WriteRelationshipsRequest {
                updates: vec![RelationshipUpdate {
                    operation: RelOp::Delete as i32,
                    relationship: Some(rel("doc2", "alice")),
                }],
                optional_preconditions: Vec::new(),
                optional_transaction_metadata: None,
            })
            .await
            .expect("delete alice->doc2");

        // 5. Re-read with subject=alice: now 1 row (doc1 only).
        let alice_after = client
            .read_relationships(ReadRelationshipsRequest {
                consistency: consistency(),
                relationship_filter: Some(RelationshipFilter {
                    resource_type: "document".into(),
                    optional_resource_id: String::new(),
                    optional_relation: String::new(),
                    optional_resource_id_prefix: String::new(),
                    optional_subject_filter: Some(SubjectFilter {
                        subject_type: "user".into(),
                        optional_subject_id: "alice".into(),
                        optional_relation: None,
                    }),
                }),
                optional_limit: 0,
                optional_cursor: None,
            })
            .await
            .expect("read alice after delete");
        assert_eq!(
            alice_after.len(),
            1,
            "after deleting alice->doc2, only doc1 should remain"
        );
        let surviving = alice_after[0]
            .relationship
            .as_ref()
            .and_then(|r| r.resource.as_ref())
            .map(|r| r.object_id.clone());
        assert_eq!(surviving.as_deref(), Some("doc1"));
    }
}
