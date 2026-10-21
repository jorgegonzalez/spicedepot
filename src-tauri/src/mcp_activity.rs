//! Persistent, bounded activity log for MCP tool calls.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tauri::AppHandle;
use tauri_plugin_store::StoreExt;

use crate::error::{AppError, AppResult};

const STORE_FILE: &str = "mcp-activity.json";
const STORE_KEY: &str = "entries";
const MAX_ENTRIES: usize = 500;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct McpActivityEntry {
    pub id: String,
    pub requested_at: DateTime<Utc>,
    pub tool: String,
    pub connection_id: Option<String>,
    pub connection_name: Option<String>,
    pub endpoint: Option<String>,
    pub arguments: Value,
    pub status: String,
    pub duration_ms: u64,
    pub summary: Option<String>,
    pub error: Option<String>,
}

pub struct McpActivityStore {
    handle: AppHandle,
}

impl McpActivityStore {
    pub fn new(handle: AppHandle) -> AppResult<Self> {
        let store = handle
            .store(STORE_FILE)
            .map_err(|error| AppError::Store(error.to_string()))?;
        if !store.has(STORE_KEY) {
            store.set(STORE_KEY, serde_json::json!([]));
            store
                .save()
                .map_err(|error| AppError::Store(error.to_string()))?;
        }
        Ok(Self { handle })
    }

    fn store(&self) -> AppResult<std::sync::Arc<tauri_plugin_store::Store<tauri::Wry>>> {
        self.handle
            .store(STORE_FILE)
            .map_err(|error| AppError::Store(error.to_string()))
    }

    pub fn list(&self) -> AppResult<Vec<McpActivityEntry>> {
        let store = self.store()?;
        let mut entries: Vec<McpActivityEntry> = serde_json::from_value(
            store.get(STORE_KEY).unwrap_or_else(|| serde_json::json!([])),
        )?;
        entries.sort_by_key(|entry| std::cmp::Reverse(entry.requested_at));
        Ok(entries)
    }

    pub fn append(&self, entry: McpActivityEntry) -> AppResult<()> {
        let store = self.store()?;
        let mut entries: Vec<McpActivityEntry> = serde_json::from_value(
            store.get(STORE_KEY).unwrap_or_else(|| serde_json::json!([])),
        )?;
        entries.push(entry);
        if entries.len() > MAX_ENTRIES {
            entries.drain(..entries.len() - MAX_ENTRIES);
        }
        store.set(STORE_KEY, serde_json::to_value(entries)?);
        store
            .save()
            .map_err(|error| AppError::Store(error.to_string()))
    }

    pub fn clear(&self) -> AppResult<()> {
        let store = self.store()?;
        store.set(STORE_KEY, serde_json::json!([]));
        store
            .save()
            .map_err(|error| AppError::Store(error.to_string()))
    }
}
