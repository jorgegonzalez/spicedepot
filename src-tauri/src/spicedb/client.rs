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
    CheckPermissionRequest, CheckPermissionResponse, PermissionsServiceClient,
    ReadSchemaRequest, ReadSchemaResponse, SchemaServiceClient, WriteSchemaRequest,
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
}
