//! Persistent connection store.
//!
//! Connection metadata (name, endpoint, insecure flag, timestamps) is kept in
//! a JSON file managed by `tauri-plugin-store`. The bearer token is **not**
//! written there — it goes into the OS keychain via `keyring` and is keyed by
//! the connection's UUID.
//!
//! This split lets users back up / commit `connections.json` without exposing
//! credentials, and means tokens get the OS-level protection (Keychain,
//! Credential Manager, Secret Service) the platform provides.

use crate::error::{AppError, AppResult};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tauri::AppHandle;
use tauri_plugin_store::StoreExt;

const STORE_FILE: &str = "connections.json";
const STORE_KEY: &str = "connections";
const KEYRING_SERVICE: &str = "dev.spicelens.app";

/// Public-facing connection record. Token never round-trips through this struct.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Connection {
    pub id: String,
    pub name: String,
    pub endpoint: String,
    pub insecure: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// What the frontend sends on create/update.
#[derive(Clone, Debug, Deserialize)]
pub struct ConnectionInput {
    pub name: String,
    pub endpoint: String,
    pub insecure: bool,
    pub token: String,
}

pub struct ConnectionStore {
    handle: AppHandle,
}

impl ConnectionStore {
    pub fn new(handle: AppHandle) -> AppResult<Self> {
        // Touch the store so the file is created up front and we fail fast if
        // the app data dir isn't writable.
        let store = handle
            .store(STORE_FILE)
            .map_err(|e| AppError::Store(e.to_string()))?;
        if !store.has(STORE_KEY) {
            store.set(
                STORE_KEY,
                serde_json::to_value(HashMap::<String, Connection>::new())?,
            );
            store
                .save()
                .map_err(|e| AppError::Store(e.to_string()))?;
        }
        Ok(Self { handle })
    }

    fn store(&self) -> AppResult<std::sync::Arc<tauri_plugin_store::Store<tauri::Wry>>> {
        self.handle
            .store(STORE_FILE)
            .map_err(|e| AppError::Store(e.to_string()))
    }

    fn read_all(&self) -> AppResult<HashMap<String, Connection>> {
        let store = self.store()?;
        let val = store
            .get(STORE_KEY)
            .unwrap_or(serde_json::json!({}));
        let map: HashMap<String, Connection> = serde_json::from_value(val)?;
        Ok(map)
    }

    fn write_all(&self, map: &HashMap<String, Connection>) -> AppResult<()> {
        let store = self.store()?;
        store.set(STORE_KEY, serde_json::to_value(map)?);
        store
            .save()
            .map_err(|e| AppError::Store(e.to_string()))?;
        Ok(())
    }

    pub fn list(&self) -> AppResult<Vec<Connection>> {
        let mut v: Vec<Connection> = self.read_all()?.into_values().collect();
        v.sort_by(|a, b| a.created_at.cmp(&b.created_at));
        Ok(v)
    }

    pub fn get(&self, id: &str) -> AppResult<Connection> {
        self.read_all()?
            .remove(id)
            .ok_or_else(|| AppError::NotFound(id.to_string()))
    }

    pub fn create(&self, input: ConnectionInput) -> AppResult<Connection> {
        let id = uuid::Uuid::new_v4().to_string();
        let now = Utc::now();
        let conn = Connection {
            id: id.clone(),
            name: input.name,
            endpoint: input.endpoint,
            insecure: input.insecure,
            created_at: now,
            updated_at: now,
        };
        let mut map = self.read_all()?;
        map.insert(id.clone(), conn.clone());
        self.write_all(&map)?;
        // Token may legitimately be empty (anonymous SpiceDB instances exist).
        if !input.token.is_empty() {
            keyring::Entry::new(KEYRING_SERVICE, &id)?.set_password(&input.token)?;
        }
        Ok(conn)
    }

    pub fn update(&self, id: &str, input: ConnectionInput) -> AppResult<Connection> {
        let mut map = self.read_all()?;
        let existing = map
            .get_mut(id)
            .ok_or_else(|| AppError::NotFound(id.to_string()))?;
        existing.name = input.name;
        existing.endpoint = input.endpoint;
        existing.insecure = input.insecure;
        existing.updated_at = Utc::now();
        let updated = existing.clone();
        self.write_all(&map)?;
        // Empty token from the form means "leave unchanged".
        if !input.token.is_empty() {
            keyring::Entry::new(KEYRING_SERVICE, id)?.set_password(&input.token)?;
        }
        Ok(updated)
    }

    pub fn delete(&self, id: &str) -> AppResult<()> {
        let mut map = self.read_all()?;
        if map.remove(id).is_none() {
            return Err(AppError::NotFound(id.to_string()));
        }
        self.write_all(&map)?;
        // Best-effort token removal — ignore "no entry" type errors.
        if let Ok(entry) = keyring::Entry::new(KEYRING_SERVICE, id) {
            let _ = entry.delete_credential();
        }
        Ok(())
    }

    pub fn token(&self, id: &str) -> AppResult<String> {
        match keyring::Entry::new(KEYRING_SERVICE, id)?.get_password() {
            Ok(t) => Ok(t),
            Err(keyring::Error::NoEntry) => Ok(String::new()),
            Err(e) => Err(AppError::from(e)),
        }
    }
}
