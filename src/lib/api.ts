import { invoke } from "./tauri";

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
};
