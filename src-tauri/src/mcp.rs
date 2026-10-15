//! Local, read-only MCP server for agent clients.
//!
//! The desktop app owns MCP service access. Agent-launched stdio processes
//! bridge to a private local socket, so tools work only while the app is open.

use crate::connections::Connection;
use crate::connections::ConnectionStore;
use crate::mcp_activity::{McpActivityEntry, McpActivityStore};
use crate::spicedb::client::{DialConfig, SpiceDbClient};
use crate::tunnels::TunnelManager;
use crate::spicedb::proto::{
    authzed::api::v1::{consistency::Requirement, subject_filter::RelationFilter},
    CheckPermissionRequest, Consistency, LookupResourcesRequest, LookupSubjectsRequest,
    ObjectReference, ReadRelationshipsRequest, RelationshipFilter, SubjectFilter, SubjectReference,
};
use interprocess::local_socket::{
    tokio::{prelude::*, Stream},
    GenericFilePath, ListenerOptions, ToFsName,
};
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use std::time::Instant;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::sync::Mutex;

const APP_ID: &str = "dev.spicelens.app";
const MAX_RESULTS: u32 = 500;

async fn read_connections(store: &Arc<Mutex<ConnectionStore>>) -> Result<Vec<Connection>, String> {
    store.lock().await.list().map_err(|e| e.to_string())
}

async fn client_for(
    store: &Arc<Mutex<ConnectionStore>>,
    tunnels: &Arc<Mutex<TunnelManager>>,
    connection_ref: &str,
) -> Result<(Connection, SpiceDbClient), String> {
    let connections = read_connections(store).await?;
    let matching: Vec<_> = connections
        .into_iter()
        .filter(|c| c.id == connection_ref || c.name == connection_ref)
        .collect();
    let connection = match matching.as_slice() {
        [one] => one.clone(),
        [] => {
            return Err(format!(
                "No saved connection named or identified by '{connection_ref}'"
            ))
        }
        _ => {
            return Err(format!(
                "More than one saved connection is named '{connection_ref}'; use its connection ID"
            ))
        }
    };
    tunnels.lock().await.ensure_started(&connection).await.map_err(|e| e.to_string())?;
    let id = connection.id.clone();
    let token = crate::connections::read_token(&id)
        .await
        .map_err(|error| format!("Could not read the saved token from the OS keychain: {error}"))?;
    let client = SpiceDbClient::connect(DialConfig {
        endpoint: connection.endpoint.clone(),
        insecure: connection.insecure,
        token,
    })
    .await
    .map_err(|e| e.to_string())?;
    Ok((connection, client))
}

pub fn run_stdio() -> anyhow::Result<()> {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?
        .block_on(bridge_stdio())
}

fn socket_name(path: &Path) -> anyhow::Result<interprocess::local_socket::Name<'static>> {
    #[cfg(unix)]
    {
        Ok(path
            .to_string_lossy()
            .into_owned()
            .to_fs_name::<GenericFilePath>()?)
    }
    #[cfg(windows)]
    {
        use interprocess::local_socket::{GenericNamespaced, ToNsName};
        Ok("spicedepot-mcp"
            .to_string()
            .to_ns_name::<GenericNamespaced>()?)
    }
}

pub fn start_server(
    store: Arc<Mutex<ConnectionStore>>,
    activity: Arc<Mutex<McpActivityStore>>,
    tunnels: Arc<Mutex<TunnelManager>>,
    data_dir: &Path,
) -> anyhow::Result<tauri::async_runtime::JoinHandle<()>> {
    std::fs::create_dir_all(data_dir)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(data_dir, std::fs::Permissions::from_mode(0o700))?;
    }
    let socket_path = data_dir.join("spicedepot-mcp.sock");
    #[cfg(unix)]
    prepare_socket_path(&socket_path)?;
    let name = socket_name(&socket_path)?;
    let options = ListenerOptions::new().name(name);
    #[cfg(target_os = "linux")]
    let options = {
        use interprocess::os::unix::local_socket::ListenerOptionsExt;
        options.mode(0o600)
    };
    Ok(tauri::async_runtime::spawn(async move {
        #[cfg(windows)]
        let listener_result = {
            use interprocess::os::windows::{
                local_socket::ListenerOptionsExt, security_descriptor::SecurityDescriptor,
            };
            // Owner Rights grants access to the account that created this pipe.
            (|| {
                let sddl = widestring::U16CString::from_str("D:P(A;;GA;;;OW)")
                    .map_err(std::io::Error::other)?;
                let descriptor = SecurityDescriptor::deserialize(sddl.as_ucstr())?;
                options
                    .security_descriptor(descriptor.borrow())
                    .create_tokio()
            })()
        };
        #[cfg(not(windows))]
        let listener_result = options.create_tokio();
        let listener = match listener_result {
            Ok(listener) => listener,
            Err(error) => {
                tracing::error!(%error, "Could not start SpiceDepot's local MCP service");
                return;
            }
        };
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Err(error) = std::fs::set_permissions(&socket_path, std::fs::Permissions::from_mode(0o600)) {
                tracing::error!(%error, "Could not restrict MCP socket access");
                return;
            }
        }
        loop {
            match listener.accept().await {
                Ok(stream) => {
                    let store = store.clone();
                    let activity = activity.clone();
                    let tunnels = tunnels.clone();
                    tauri::async_runtime::spawn(async move {
                        if let Err(error) = serve_stream(stream, store, activity, tunnels).await {
                            tracing::debug!(%error, "MCP client disconnected");
                        }
                    });
                }
                Err(error) => {
                    tracing::error!(%error, "MCP local socket accept failed");
                    break;
                }
            }
        }
    }))
}

#[cfg(unix)]
fn prepare_socket_path(socket_path: &Path) -> anyhow::Result<()> {
    use std::io::ErrorKind;
    use std::os::unix::net::UnixStream;

    if !socket_path.exists() {
        return Ok(());
    }
    match UnixStream::connect(socket_path) {
        Ok(_) => anyhow::bail!(
            "Another SpiceDepot instance already owns the local MCP socket at {}",
            socket_path.display()
        ),
        Err(error) if matches!(error.kind(), ErrorKind::ConnectionRefused | ErrorKind::NotFound) => {
            std::fs::remove_file(socket_path)?;
            Ok(())
        }
        Err(error) => Err(error.into()),
    }
}

async fn bridge_stdio() -> anyhow::Result<()> {
    let path = dirs::data_dir()
        .ok_or_else(|| anyhow::anyhow!("Could not find the application data directory"))?
        .join(APP_ID)
        .join("spicedepot-mcp.sock");
    let name = socket_name(&path)?;
    let stream = Stream::connect(name).await.map_err(|error| {
        anyhow::anyhow!("Could not connect to SpiceDepot. Open the SpiceDepot app first. {error}")
    })?;
    let (mut socket_read, mut socket_write) = tokio::io::split(stream);
    let mut stdin = tokio::io::stdin();
    let mut stdout = tokio::io::stdout();
    let input = tokio::io::copy(&mut stdin, &mut socket_write);
    let output = tokio::io::copy(&mut socket_read, &mut stdout);
    tokio::try_join!(input, output)?;
    Ok(())
}

async fn serve_stream(
    stream: Stream,
    store: Arc<Mutex<ConnectionStore>>,
    activity: Arc<Mutex<McpActivityStore>>,
    tunnels: Arc<Mutex<TunnelManager>>,
) -> anyhow::Result<()> {
    let (reader, mut writer) = tokio::io::split(stream);
    let mut reader = BufReader::new(reader);
    let mut line = String::new();
    loop {
        line.clear();
        if (&mut reader).take(4 * 1024 * 1024 + 1).read_line(&mut line).await? == 0 {
            break;
        }
        if line.len() > 4 * 1024 * 1024 {
            break;
        }
        let parsed: Result<Value, _> = serde_json::from_str(&line);
        let response = match parsed {
            Ok(request) => handle(request, &store, &activity, &tunnels).await,
            Err(error) => Some(json!({
                "jsonrpc":"2.0", "id":null,
                "error":{"code":-32700,"message":format!("Parse error: {error}")}
            })),
        };
        if let Some(response) = response {
            let mut encoded = serde_json::to_vec(&response)?;
            encoded.push(b'\n');
            writer.write_all(&encoded).await?;
            writer.flush().await?;
        }
    }
    Ok(())
}

async fn handle(
    request: Value,
    store: &Arc<Mutex<ConnectionStore>>,
    activity: &Arc<Mutex<McpActivityStore>>,
    tunnels: &Arc<Mutex<TunnelManager>>,
) -> Option<Value> {
    let id = request.get("id")?.clone();
    let method = request.get("method")?.as_str()?;
    let params = request.get("params").cloned().unwrap_or_else(|| json!({}));
    let result: Result<Value, String> = match method {
        "initialize" => Ok(json!({
            "protocolVersion": match params.get("protocolVersion").and_then(Value::as_str) {
                Some("2025-11-25") => "2025-11-25",
                Some("2025-06-18") => "2025-06-18",
                Some("2025-03-26") => "2025-03-26",
                _ => "2025-11-25",
            },
            "capabilities": {"tools": {"listChanged": false}},
            "serverInfo": {"name": "spicedepot", "version": env!("CARGO_PKG_VERSION")},
            "instructions": "SpiceDepot exposes read-only SpiceDB tools. Choose a saved connection explicitly. No schema or relationship writes are available through this MCP server."
        })),
        "ping" => Ok(json!({})),
        "tools/list" => Ok(json!({"tools": tool_definitions()})),
        "tools/call" => {
            let started = Instant::now();
            let requested_at = chrono::Utc::now();
            let outcome = call_tool(&params, store, tunnels).await;
            record_activity(
                activity,
                store,
                &params,
                &outcome,
                requested_at,
                started.elapsed().as_millis() as u64,
            )
            .await;
            Ok(match outcome {
                Ok(result) => result,
                Err(error) => json!({"content":[{"type":"text","text":error}],"isError":true}),
            })
        }
        _ => {
            return Some(
                json!({"jsonrpc":"2.0", "id":id, "error":{"code":-32601,"message":"Method not found"}}),
            )
        }
    };
    Some(match result {
        Ok(result) => json!({"jsonrpc":"2.0", "id":id, "result":result}),
        Err(error) => json!({"jsonrpc":"2.0", "id":id, "error":{"code":-32603,"message":error}}),
    })
}

async fn record_activity(
    activity: &Arc<Mutex<McpActivityStore>>,
    connections: &Arc<Mutex<ConnectionStore>>,
    params: &Value,
    outcome: &Result<Value, String>,
    requested_at: chrono::DateTime<chrono::Utc>,
    duration_ms: u64,
) {
    let tool = params
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or("unknown")
        .to_string();
    let arguments = params
        .get("arguments")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let allowed = ["connection", "resource_type", "resource_id", "permission", "relation", "subject_type", "subject_id", "subject_relation", "limit"];
    let logged = arguments.as_object().map(|map| map.iter()
        .filter(|(key, _)| allowed.contains(&key.as_str()))
        .map(|(key, value)| (key.clone(), value.clone())).collect::<serde_json::Map<String, Value>>())
        .unwrap_or_default();
    let safe_arguments = sanitize_arguments(&Value::Object(logged));
    let connection_ref = arguments.get("connection").and_then(Value::as_str);
    let connection = if let Some(reference) = connection_ref {
        read_connections(connections)
            .await
            .ok()
            .and_then(|rows| rows.into_iter().find(|row| row.id == reference || row.name == reference))
    } else {
        None
    };
    let (status, summary, error) = match outcome {
        Ok(result) => ("success".to_string(), summarize_result(&tool, result), None),
        Err(_) => (
            "error".to_string(),
            None,
            Some("Request failed; inspect the tool response for details.".to_string()),
        ),
    };
    let entry = McpActivityEntry {
        id: uuid::Uuid::new_v4().to_string(),
        requested_at,
        tool,
        connection_id: connection.as_ref().map(|value| value.id.clone()),
        connection_name: connection
            .as_ref()
            .map(|value| value.name.clone())
            ,
        endpoint: connection.map(|value| value.endpoint),
        arguments: safe_arguments,
        status,
        duration_ms,
        summary,
        error,
    };
    if let Err(error) = activity.lock().await.append(entry) {
        tracing::warn!(%error, "Could not persist MCP activity");
    }
}

fn sanitize_arguments(value: &Value) -> Value {
    match value {
        Value::Object(map) => Value::Object(
            map.iter()
                .map(|(key, value)| {
                    let normalized = key.to_ascii_lowercase();
                    let safe = if ["token", "authorization", "password", "secret", "preshared_key"]
                        .iter()
                        .any(|sensitive| normalized.contains(sensitive))
                    {
                        Value::String("[REDACTED]".into())
                    } else {
                        sanitize_arguments(value)
                    };
                    (key.clone(), safe)
                })
                .collect(),
        ),
        Value::Array(items) => Value::Array(items.iter().map(sanitize_arguments).collect()),
        _ => value.clone(),
    }
}

fn summarize_result(tool: &str, result: &Value) -> Option<String> {
    let content = result.get("structuredContent")?;
    match tool {
        "list_connections" => content
            .get("connections")
            .and_then(Value::as_array)
            .map(|items| format!("{} connection(s)", items.len())),
        "read_schema" => content
            .get("schema")
            .and_then(Value::as_str)
            .map(|schema| format!("{} schema character(s)", schema.len())),
        "check_permission" => content
            .get("permissionship")
            .and_then(Value::as_str)
            .map(str::to_owned),
        "lookup_resources" | "lookup_subjects" | "read_relationships" => content
            .get("items")
            .and_then(Value::as_array)
            .map(|items| format!("{} result(s)", items.len())),
        _ => None,
    }
}

fn tool_definitions() -> Value {
    json!([
      {"name":"list_connections","description":"List saved SpiceDepot connections. Tokens are never returned.","inputSchema":{"type":"object","properties":{},"additionalProperties":false}},
      {"name":"read_schema","description":"Read the SpiceDB schema for one saved connection.","inputSchema":{"type":"object","properties":{"connection":{"type":"string","description":"Saved connection name or ID."}},"required":["connection"],"additionalProperties":false}},
      {"name":"check_permission","description":"Check whether a subject has a permission on a resource.","inputSchema":{"type":"object","properties":{"connection":{"type":"string"},"resource_type":{"type":"string"},"resource_id":{"type":"string"},"permission":{"type":"string"},"subject_type":{"type":"string"},"subject_id":{"type":"string"},"subject_relation":{"type":"string"}},"required":["connection","resource_type","resource_id","permission","subject_type","subject_id"],"additionalProperties":false}},
      {"name":"lookup_resources","description":"Find resources a subject can access. Results are capped at 500.","inputSchema":{"type":"object","properties":{"connection":{"type":"string"},"resource_type":{"type":"string"},"permission":{"type":"string"},"subject_type":{"type":"string"},"subject_id":{"type":"string"},"subject_relation":{"type":"string"},"limit":{"type":"integer","minimum":1,"maximum":500}},"required":["connection","resource_type","permission","subject_type","subject_id"],"additionalProperties":false}},
      {"name":"lookup_subjects","description":"Find subjects with a permission on one resource. Results are capped at 500.","inputSchema":{"type":"object","properties":{"connection":{"type":"string"},"resource_type":{"type":"string"},"resource_id":{"type":"string"},"permission":{"type":"string"},"subject_type":{"type":"string"},"subject_relation":{"type":"string"},"limit":{"type":"integer","minimum":1,"maximum":500}},"required":["connection","resource_type","resource_id","permission","subject_type"],"additionalProperties":false}},
      {"name":"read_relationships","description":"Read relationships matching filters. Results are capped at 500.","inputSchema":{"type":"object","properties":{"connection":{"type":"string"},"resource_type":{"type":"string"},"resource_id":{"type":"string"},"relation":{"type":"string"},"subject_type":{"type":"string"},"subject_id":{"type":"string"},"subject_relation":{"type":"string"},"limit":{"type":"integer","minimum":1,"maximum":500}},"required":["connection","resource_type"],"additionalProperties":false}}
    ])
}

#[derive(Deserialize)]
struct CallArgs {
    connection: String,
    #[serde(flatten)]
    other: HashMap<String, Value>,
}

async fn call_tool(
    params: &Value,
    store: &Arc<Mutex<ConnectionStore>>,
    tunnels: &Arc<Mutex<TunnelManager>>,
) -> Result<Value, String> {
    let name = params
        .get("name")
        .and_then(Value::as_str)
        .ok_or("Missing tool name")?;
    let arguments = params
        .get("arguments")
        .cloned()
        .unwrap_or_else(|| json!({}));
    if name == "list_connections" {
        let rows: Vec<_> = read_connections(store)
            .await?
            .into_iter()
            .map(|c| {
                json!({
                    "id": c.id, "name": c.name, "endpoint": c.endpoint, "tls": !c.insecure
                })
            })
            .collect();
        return Ok(tool_result(&json!({"connections": rows})));
    }

    let args: CallArgs =
        serde_json::from_value(arguments).map_err(|e| format!("Invalid arguments: {e}"))?;
    let get = |key: &str| -> Result<String, String> {
        args.other
            .get(key)
            .and_then(Value::as_str)
            .map(str::to_owned)
            .ok_or_else(|| format!("Missing string argument '{key}'"))
    };
    let optional = |key: &str| -> Option<String> {
        args.other
            .get(key)
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty())
            .map(str::to_owned)
    };
    let limit = || -> Result<u32, String> {
        let value = args
            .other
            .get("limit")
            .and_then(Value::as_u64)
            .unwrap_or(100);
        if value == 0 || value > MAX_RESULTS as u64 {
            return Err(format!("limit must be between 1 and {MAX_RESULTS}"));
        }
        Ok(value as u32)
    };
    let (_, client) = client_for(store, tunnels, &args.connection).await?;
    let fully_consistent = || {
        Some(Consistency {
            requirement: Some(Requirement::FullyConsistent(true)),
        })
    };

    let result = match name {
        "read_schema" => match client.read_schema().await {
            Ok(response) => json!({"schema": response.schema_text}),
            Err(crate::error::AppError::Grpc {
                code: tonic::Code::NotFound,
                ..
            }) => json!({"schema": ""}),
            Err(error) => return Err(error.to_string()),
        },
        "check_permission" => {
            let resource_type = get("resource_type")?;
            let resource_id = get("resource_id")?;
            let permission = get("permission")?;
            let subject_type = get("subject_type")?;
            let subject_id = get("subject_id")?;
            let subject_relation = optional("subject_relation").unwrap_or_default();
            let response = client
                .check_permission(CheckPermissionRequest {
                    consistency: fully_consistent(),
                    resource: Some(ObjectReference {
                        object_type: resource_type,
                        object_id: resource_id,
                    }),
                    permission,
                    subject: Some(SubjectReference {
                        object: Some(ObjectReference {
                            object_type: subject_type,
                            object_id: subject_id,
                        }),
                        optional_relation: subject_relation,
                    }),
                    context: None,
                    with_tracing: false,
                })
                .await
                .map_err(|e| e.to_string())?;
            json!({"permissionship": format!("{:?}", response.permissionship()), "checked_at": response.checked_at.map(|t| t.token), "missing_context": response.partial_caveat_info.map(|p| p.missing_required_context).unwrap_or_default()})
        }
        "lookup_resources" => {
            let resource_type = get("resource_type")?;
            let permission = get("permission")?;
            let subject_type = get("subject_type")?;
            let subject_id = get("subject_id")?;
            let subject_relation = optional("subject_relation").unwrap_or_default();
            let response = client
                .lookup_resources(LookupResourcesRequest {
                    consistency: fully_consistent(),
                    resource_object_type: resource_type,
                    permission,
                    subject: Some(SubjectReference {
                        object: Some(ObjectReference {
                            object_type: subject_type,
                            object_id: subject_id,
                        }),
                        optional_relation: subject_relation,
                    }),
                    context: None,
                    optional_limit: limit()?,
                    optional_cursor: None,
                    with_debug: false,
                })
                .await
                .map_err(|e| e.to_string())?;
            let items: Vec<_> = response.into_iter().map(|r| json!({"resource_id":r.resource_object_id,"permissionship":crate::spicedb::proto::authzed::api::v1::LookupPermissionship::try_from(r.permissionship).map(|p|format!("{p:?}")).unwrap_or_else(|_|"Unspecified".into()),"missing_context":r.partial_caveat_info.map(|p|p.missing_required_context).unwrap_or_default()})).collect();
            json!({"items":items})
        }
        "lookup_subjects" => {
            let resource_type = get("resource_type")?;
            let resource_id = get("resource_id")?;
            let permission = get("permission")?;
            let subject_type = get("subject_type")?;
            let subject_relation = optional("subject_relation").unwrap_or_default();
            let result_limit = limit()? as usize;
            let (response, truncated) = client
                .lookup_subjects_capped(
                    LookupSubjectsRequest {
                        consistency: fully_consistent(),
                        resource: Some(ObjectReference {
                            object_type: resource_type,
                            object_id: resource_id,
                        }),
                        permission,
                        subject_object_type: subject_type,
                        optional_subject_relation: subject_relation,
                        context: None,
                        // SpiceDB versions before concrete-limit support reject
                        // this field, so enforce the cap while draining instead.
                        optional_concrete_limit: 0,
                        optional_cursor: None,
                        wildcard_option: 0,
                    },
                    result_limit,
                )
                .await
                .map_err(|e| e.to_string())?;
            let items: Vec<_> = response.into_iter().filter_map(|r| r.subject.map(|s| json!({"subject_id":s.subject_object_id,"permissionship":crate::spicedb::proto::authzed::api::v1::LookupPermissionship::try_from(s.permissionship).map(|p|format!("{p:?}")).unwrap_or_else(|_|"Unspecified".into()),"missing_context":s.partial_caveat_info.map(|p|p.missing_required_context).unwrap_or_default()}))).collect();
            json!({"items":items,"truncated":truncated})
        }
        "read_relationships" => {
            let resource_type = get("resource_type")?;
            let subject_filter = optional("subject_type").map(|subject_type| SubjectFilter {
                subject_type,
                optional_subject_id: optional("subject_id").unwrap_or_default(),
                optional_relation: optional("subject_relation")
                    .map(|relation| RelationFilter { relation }),
            });
            let response = client
                .read_relationships(ReadRelationshipsRequest {
                    consistency: fully_consistent(),
                    relationship_filter: Some(RelationshipFilter {
                        resource_type,
                        optional_resource_id: optional("resource_id").unwrap_or_default(),
                        optional_resource_id_prefix: String::new(),
                        optional_relation: optional("relation").unwrap_or_default(),
                        optional_subject_filter: subject_filter,
                    }),
                    optional_limit: limit()?,
                    optional_cursor: None,
                })
                .await
                .map_err(|e| e.to_string())?;
            let items: Vec<_> = response.into_iter().filter_map(|r| r.relationship).map(|r| {
                let resource = r.resource.unwrap_or_default();
                let subject = r.subject.unwrap_or_default();
                let subject_object = subject.object.unwrap_or_default();
                json!({"resource_type":resource.object_type,"resource_id":resource.object_id,"relation":r.relation,"subject_type":subject_object.object_type,"subject_id":subject_object.object_id,"subject_relation":subject.optional_relation,"caveat":r.optional_caveat.map(|c|c.caveat_name)})
            }).collect();
            json!({"items":items})
        }
        _ => return Err(format!("Unknown tool '{name}'")),
    };
    Ok(tool_result(&result))
}

fn tool_result(value: &Value) -> Value {
    json!({"content":[{"type":"text","text":serde_json::to_string_pretty(value).unwrap_or_else(|_| "{}".into())}],"structuredContent":value,"isError":false})
}

#[cfg(test)]
mod security_tests {
    use super::*;

    #[test]
    fn sensitive_arguments_are_recursively_redacted() {
        let result = sanitize_arguments(&json!({
            "token": "test-only-secret", "nested": [{"Authorization": "test-only-secret"}],
            "resource_id": "example"
        }));
        assert!(!result.to_string().contains("test-only-secret"));
        assert_eq!(result["resource_id"], "example");
    }
}
