import { Channel, invoke } from "./tauri";

/** Mirrors the `Connection` struct in `src-tauri/src/connections.rs`. */
export interface Connection {
  id: string;
  name: string;
  endpoint: string;
  insecure: boolean;
  created_at: string;
  updated_at: string;
}

export interface ConnectionInput {
  name: string;
  endpoint: string;
  insecure: boolean;
  /** Bearer token. Stored in OS keychain, never returned by `list_connections`. */
  token: string;
}

export interface SchemaResult {
  schema_text: string;
  read_at: string | null;
}

export interface CheckPermissionInput {
  resource_type: string;
  resource_id: string;
  permission: string;
  subject_type: string;
  subject_id: string;
  subject_relation?: string;
}

export type Permissionship =
  | "has_permission"
  | "no_permission"
  | "conditional_permission"
  | "unspecified";

export interface CheckPermissionOutput {
  permissionship: Permissionship;
  checked_at: string | null;
  missing_context: string[];
}

export interface LookupResourcesInput {
  resource_type: string;
  permission: string;
  subject_type: string;
  subject_id: string;
  subject_relation?: string;
  limit?: number;
}

export interface LookupResourceItem {
  resource_id: string;
  permissionship: Permissionship;
  missing_context: string[];
}

export interface LookupResourcesOutput {
  items: LookupResourceItem[];
  looked_up_at: string | null;
}

export interface LookupSubjectsInput {
  resource_type: string;
  resource_id: string;
  permission: string;
  subject_type: string;
  subject_relation?: string;
  limit?: number;
}

export interface LookupSubjectItem {
  subject_id: string;
  permissionship: Permissionship;
  excluded_subject_ids: string[];
  missing_context: string[];
}

export interface LookupSubjectsOutput {
  items: LookupSubjectItem[];
  looked_up_at: string | null;
}

export interface RelationshipFilterInput {
  resource_type: string;
  resource_id?: string;
  relation?: string;
  subject_type?: string;
  subject_id?: string;
  subject_relation?: string;
}

export interface RelationshipInput {
  resource_type: string;
  resource_id: string;
  relation: string;
  subject_type: string;
  subject_id: string;
  subject_relation?: string;
}

export interface RelationshipRow {
  resource_type: string;
  resource_id: string;
  relation: string;
  subject_type: string;
  subject_id: string;
  subject_relation: string | null;
  caveat_name: string | null;
}

export interface ReadRelationshipsOutput {
  items: RelationshipRow[];
  read_at: string | null;
}

export type WriteOperation = "create" | "touch" | "delete";

export interface BulkDeleteOutput {
  deleted_count: number;
  progress: "complete" | "partial" | "unspecified";
  deleted_at: string | null;
}

export type WatchEvent =
  | {
      kind: "update";
      operation: WriteOperation;
      relationship: RelationshipRow;
      at: string | null;
    }
  | { kind: "schema_changed"; at: string | null }
  | { kind: "ended" }
  | { kind: "error"; message: string };

export interface WatchStartInput {
  object_types: string[];
}

export const api = {
  // Connections
  listConnections: () => invoke<Connection[]>("list_connections"),
  getConnection: (id: string) =>
    invoke<Connection>("get_connection", { id }),
  createConnection: (input: ConnectionInput) =>
    invoke<Connection>("create_connection", { input }),
  updateConnection: (id: string, input: ConnectionInput) =>
    invoke<Connection>("update_connection", { id, input }),
  deleteConnection: (id: string) =>
    invoke<void>("delete_connection", { id }),
  testConnection: (id: string) =>
    invoke<string>("test_connection", { id }),

  // Schema
  readSchema: (connectionId: string) =>
    invoke<SchemaResult>("read_schema", { connectionId }),
  writeSchema: (connectionId: string, schema: string) =>
    invoke<void>("write_schema", { connectionId, schema }),

  // Permissions
  checkPermission: (connectionId: string, input: CheckPermissionInput) =>
    invoke<CheckPermissionOutput>("check_permission", {
      connectionId,
      input,
    }),

  // Lookup
  lookupResources: (connectionId: string, input: LookupResourcesInput) =>
    invoke<LookupResourcesOutput>("lookup_resources", {
      connectionId,
      input,
    }),
  lookupSubjects: (connectionId: string, input: LookupSubjectsInput) =>
    invoke<LookupSubjectsOutput>("lookup_subjects", {
      connectionId,
      input,
    }),

  // Relationships
  readRelationships: (
    connectionId: string,
    filter: RelationshipFilterInput,
    limit?: number,
  ) =>
    invoke<ReadRelationshipsOutput>("read_relationships", {
      connectionId,
      filter,
      limit,
    }),
  writeRelationship: (
    connectionId: string,
    operation: WriteOperation,
    relationship: RelationshipInput,
  ) =>
    invoke<void>("write_relationship", {
      connectionId,
      operation,
      relationship,
    }),
  bulkDeleteRelationships: (
    connectionId: string,
    filter: RelationshipFilterInput,
    limit?: number,
  ) =>
    invoke<BulkDeleteOutput>("bulk_delete_relationships", {
      connectionId,
      filter,
      limit,
    }),

  // Watch
  watchStart: (
    connectionId: string,
    input: WatchStartInput,
    onEvent: Channel<WatchEvent>,
  ) =>
    invoke<string>("watch_start", {
      connectionId,
      input,
      onEvent,
    }),
  watchStop: (watchId: string) =>
    invoke<void>("watch_stop", { watchId }),
};
