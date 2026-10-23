//! # Rune Store SQLite Repository
//!
//! Provides local database persistence for custom and imported rune pages
//! using embedded SQLite via the `rusqlite` crate.

use crate::models::{RuneSetRecord, RuneSetSummary};
use rusqlite::{params, Connection};
use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;

/// Embedded SQLite database manager for storing and querying rune pages.
pub struct RuneStore {
    db_path: PathBuf,
}

impl RuneStore {
    /// Creates a new `RuneStore` instance, ensuring the database file and directory exist.
    ///
    /// The SQLite database is stored at `%LOCALAPPDATA%\lcu_companion\lcu_companion.db`.
    ///
    /// # Errors
    /// Returns `Err(String)` if directory creation or schema initialization fails.
    pub fn new() -> Result<Self, String> {
        let mut db_dir = dirs::data_local_dir().unwrap_or_else(|| PathBuf::from("."));
        db_dir.push("lcu_companion");
        fs::create_dir_all(&db_dir).map_err(|e| format!("Failed creating db directory: {}", e))?;

        let mut db_path = db_dir;
        db_path.push("lcu_companion.db");

        let store = Self { db_path };
        store.initialize_schema()?;
        Ok(store)
    }

    /// Opens a connection to the SQLite database.
    fn connect(&self) -> Result<Connection, String> {
        let conn = Connection::open(&self.db_path)
            .map_err(|e| format!("Failed connecting to SQLite at {:?}: {}", self.db_path, e))?;
        let _ = conn.busy_timeout(std::time::Duration::from_secs(5));
        let _ = conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA synchronous = NORMAL;
             PRAGMA temp_store = MEMORY;
             PRAGMA mmap_size = 268435456;
             PRAGMA cache_size = -64000;",
        );
        Ok(conn)
    }

    /// Initializes the `runesets` and `favorite_champions` table schema if it does not already exist.
    pub fn initialize_schema(&self) -> Result<(), String> {
        let conn = self.connect()?;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS runesets (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                summoner_puuid TEXT,
                champion_id INTEGER,
                game_id INTEGER,
                rune_page_id INTEGER,
                rune_page_name TEXT,
                primary_style_id INTEGER,
                sub_style_id INTEGER,
                selected_perk_ids_json TEXT NOT NULL,
                source TEXT NOT NULL,
                role INTEGER NOT NULL DEFAULT 1,
                created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            )",
            [],
        )
        .map_err(|e| format!("Failed creating runesets schema: {}", e))?;

        // Migration check for existing SQLite databases created prior to role column addition
        let has_role_col: bool = conn
            .prepare("PRAGMA table_info(runesets)")
            .and_then(|mut stmt| {
                let mut rows = stmt.query([])?;
                while let Some(row) = rows.next()? {
                    let col_name: String = row.get(1)?;
                    if col_name == "role" {
                        return Ok(true);
                    }
                }
                Ok(false)
            })
            .unwrap_or(false);

        if !has_role_col {
            let _ = conn.execute(
                "ALTER TABLE runesets ADD COLUMN role INTEGER NOT NULL DEFAULT 1",
                [],
            );
        }

        conn.execute(
            "CREATE TABLE IF NOT EXISTS favorite_champions (
                champion_id INTEGER PRIMARY KEY,
                created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            )",
            [],
        )
        .map_err(|e| format!("Failed creating favorite_champions schema: {}", e))?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS app_settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL,
                updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            )",
            [],
        )
        .map_err(|e| format!("Failed creating app_settings schema: {}", e))?;

        Ok(())
    }

    /// Inserts a new rune page into SQLite and returns the generated row ID.
    ///
    /// # Arguments
    /// * `rune_page_name` - Display name of the rune page.
    /// * `primary_style_id` - Primary style tree ID (e.g. `8000`).
    /// * `sub_style_id` - Secondary style tree ID (e.g. `8100`).
    /// * `selected_perk_ids` - Array of perk and stat mod IDs.
    /// * `summoner_puuid` - Optional owner summoner PUUID.
    /// * `champion_id` - Optional champion association (defaults to 0).
    /// * `role` - Lane / role identifier (e.g. 0=Top, 1=Jungle, 2=Mid, 3=ADC, 4=Support).
    /// * `game_id` - Optional associated match ID.
    /// * `rune_page_id` - Original client page ID if imported.
    /// * `source` - Origin descriptor (e.g. `league_client`, `custom`).
    pub fn save_runeset(
        &self,
        rune_page_name: &str,
        primary_style_id: Option<i64>,
        sub_style_id: Option<i64>,
        selected_perk_ids: &[i64],
        summoner_puuid: Option<&str>,
        champion_id: i64,
        role: u32,
        game_id: Option<i64>,
        rune_page_id: Option<i64>,
        source: &str,
    ) -> Result<i64, String> {
        let conn = self.connect()?;
        let perks_json = serde_json::to_string(selected_perk_ids)
            .map_err(|e| format!("Failed serializing perks: {}", e))?;

        conn.execute(
            "INSERT INTO runesets (
                summoner_puuid,
                champion_id,
                game_id,
                rune_page_id,
                rune_page_name,
                primary_style_id,
                sub_style_id,
                selected_perk_ids_json,
                source,
                role
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                summoner_puuid,
                champion_id,
                game_id,
                rune_page_id,
                rune_page_name,
                primary_style_id,
                sub_style_id,
                perks_json,
                source,
                role,
            ],
        )
        .map_err(|e| format!("Failed inserting runeset into SQLite: {}", e))?;

        Ok(conn.last_insert_rowid())
    }

    /// Fetches a complete [`RuneSetRecord`] by its primary key ID.
    pub fn get_runeset(&self, runeset_id: i64) -> Result<Option<RuneSetRecord>, String> {
        let conn = self.connect()?;
        let mut stmt = conn
            .prepare(
                "SELECT
                    id,
                    summoner_puuid,
                    COALESCE(champion_id, 0) AS champion_id,
                    game_id,
                    rune_page_id,
                    COALESCE(rune_page_name, '') AS rune_page_name,
                    primary_style_id,
                    sub_style_id,
                    selected_perk_ids_json,
                    source,
                    COALESCE(role, 1) AS role,
                    created_at
                FROM runesets
                WHERE id = ?1",
            )
            .map_err(|e| format!("Failed preparing query: {}", e))?;

        let mut rows = stmt
            .query(params![runeset_id])
            .map_err(|e| format!("Query failed: {}", e))?;

        if let Some(row) = rows.next().map_err(|e| format!("Row read failed: {}", e))? {
            let perks_json: String = row.get(8).unwrap_or_else(|_| "[]".to_string());
            let selected_perks: Vec<i64> = serde_json::from_str(&perks_json).unwrap_or_default();

            Ok(Some(RuneSetRecord {
                id: row.get(0).unwrap_or(0),
                summoner_puuid: row.get(1).ok(),
                champion_id: row.get(2).unwrap_or(0),
                game_id: row.get(3).ok(),
                rune_page_id: row.get(4).ok(),
                rune_page_name: row.get(5).unwrap_or_default(),
                primary_style_id: row.get(6).ok(),
                sub_style_id: row.get(7).ok(),
                selected_perk_ids: selected_perks,
                source: row.get(9).unwrap_or_default(),
                role: row.get(10).unwrap_or(1),
                created_at: row.get(11).unwrap_or_default(),
            }))
        } else {
            Ok(None)
        }
    }

    /// Lists recent saved rune page summaries ordered newest first (`id DESC`).
    ///
    /// # Arguments
    /// * `limit` - Maximum number of rows to return (clamped between 1 and 200).
    pub fn list_runesets(&self, limit: i64) -> Result<Vec<RuneSetSummary>, String> {
        let conn = self.connect()?;
        let safe_limit = limit.clamp(1, 200);

        let mut stmt = conn
            .prepare(
                "SELECT
                    id,
                    COALESCE(rune_page_name, '') AS rune_page_name,
                    primary_style_id,
                    sub_style_id,
                    COALESCE(champion_id, 0) AS champion_id,
                    COALESCE(role, 1) AS role,
                    source,
                    created_at
                FROM runesets
                ORDER BY id DESC
                LIMIT ?1",
            )
            .map_err(|e| format!("Failed preparing list query: {}", e))?;

        let rows = stmt
            .query_map(params![safe_limit], |row| {
                Ok(RuneSetSummary {
                    id: row.get(0)?,
                    rune_page_name: row.get(1)?,
                    primary_style_id: row.get(2)?,
                    sub_style_id: row.get(3)?,
                    champion_id: row.get(4)?,
                    role: row.get(5)?,
                    source: row.get(6)?,
                    created_at: row.get(7)?,
                })
            })
            .map_err(|e| format!("List query execution failed: {}", e))?;

        let mut result = Vec::new();
        for r in rows {
            if let Ok(item) = r {
                result.push(item);
            }
        }
        Ok(result)
    }

    /// Updates an existing saved rune page in SQLite.
    pub fn update_runeset(
        &self,
        runeset_id: i64,
        rune_page_name: &str,
        primary_style_id: Option<i64>,
        sub_style_id: Option<i64>,
        selected_perk_ids: &[i64],
        champion_id: Option<i64>,
        role: Option<u32>,
    ) -> Result<bool, String> {
        let conn = self.connect()?;
        let perks_json = serde_json::to_string(selected_perk_ids)
            .map_err(|e| format!("Failed serializing perks: {}", e))?;

        let count = conn
            .execute(
                "UPDATE runesets
                SET rune_page_name = ?1,
                    primary_style_id = ?2,
                    sub_style_id = ?3,
                    selected_perk_ids_json = ?4,
                    champion_id = COALESCE(?5, champion_id),
                    role = COALESCE(?6, role)
                WHERE id = ?7",
                params![
                    rune_page_name,
                    primary_style_id,
                    sub_style_id,
                    perks_json,
                    champion_id,
                    role,
                    runeset_id,
                ],
            )
            .map_err(|e| format!("Failed updating runeset {}: {}", runeset_id, e))?;

        Ok(count > 0)
    }

    /// Deletes a saved rune page by its ID.
    ///
    /// Returns `Ok(true)` if a row was deleted, or `Ok(false)` if the ID did not exist.
    pub fn delete_runeset(&self, runeset_id: i64) -> Result<bool, String> {
        let conn = self.connect()?;
        let count = conn
            .execute("DELETE FROM runesets WHERE id = ?1", params![runeset_id])
            .map_err(|e| format!("Failed deleting runeset {}: {}", runeset_id, e))?;
        Ok(count > 0)
    }

    /// Returns a set of champion IDs that have been marked as favorites in SQLite.
    pub fn get_favorite_champion_ids(&self) -> Result<HashSet<i64>, String> {
        let conn = self.connect()?;
        let mut stmt = conn
            .prepare("SELECT champion_id FROM favorite_champions")
            .map_err(|e| format!("Failed preparing favorite query: {}", e))?;

        let rows = stmt
            .query_map([], |row| row.get::<_, i64>(0))
            .map_err(|e| format!("Failed querying favorites: {}", e))?;

        let mut favorites = HashSet::new();
        for row in rows {
            if let Ok(id) = row {
                favorites.insert(id);
            }
        }
        Ok(favorites)
    }

    /// Toggles the favorite status for a given champion ID.
    ///
    /// Returns `Ok(true)` if the champion is now favorited, or `Ok(false)` if it was removed from favorites.
    pub fn toggle_favorite_champion(&self, champion_id: i64) -> Result<bool, String> {
        let conn = self.connect()?;
        let exists: bool = conn
            .query_row(
                "SELECT 1 FROM favorite_champions WHERE champion_id = ?1",
                params![champion_id],
                |_| Ok(true),
            )
            .unwrap_or(false);

        if exists {
            conn.execute(
                "DELETE FROM favorite_champions WHERE champion_id = ?1",
                params![champion_id],
            )
            .map_err(|e| format!("Failed removing favorite champion {}: {}", champion_id, e))?;
            Ok(false)
        } else {
            conn.execute(
                "INSERT INTO favorite_champions (champion_id) VALUES (?1)",
                params![champion_id],
            )
            .map_err(|e| format!("Failed inserting favorite champion {}: {}", champion_id, e))?;
            Ok(true)
        }
    }

    /// Gets an application setting value by key.
    pub fn get_setting(&self, key: &str) -> Result<Option<String>, String> {
        let conn = self.connect()?;
        let mut stmt = conn
            .prepare("SELECT value FROM app_settings WHERE key = ?1")
            .map_err(|e| format!("Failed preparing setting query: {}", e))?;
        let result = stmt
            .query_row(params![key], |row| row.get::<_, String>(0))
            .ok();
        Ok(result)
    }

    /// Sets or updates an application setting value by key.
    pub fn set_setting(&self, key: &str, value: &str) -> Result<(), String> {
        let conn = self.connect()?;
        conn.execute(
            "INSERT INTO app_settings (key, value, updated_at) VALUES (?1, ?2, CURRENT_TIMESTAMP)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = CURRENT_TIMESTAMP",
            params![key, value],
        )
        .map_err(|e| format!("Failed saving setting {}: {}", key, e))?;
        Ok(())
    }

    /// Retrieves the last selected champion ID persisted in SQLite.
    pub fn get_selected_champion_id(&self) -> Result<Option<i64>, String> {
        match self.get_setting("selected_champion_id")? {
            Some(val) => {
                if let Ok(id) = val.parse::<i64>() {
                    if id > 0 {
                        return Ok(Some(id));
                    }
                }
                Ok(None)
            }
            None => Ok(None),
        }
    }

    /// Persists the last selected champion ID in SQLite.
    pub fn set_selected_champion_id(&self, champion_id: Option<i64>) -> Result<(), String> {
        let val = champion_id.unwrap_or(0).to_string();
        self.set_setting("selected_champion_id", &val)
    }

    /// Retrieves the last selected rune key persisted in SQLite (e.g. "saved:1" or "client:12345").
    pub fn get_selected_rune_key(&self) -> Result<Option<String>, String> {
        match self.get_setting("selected_rune_key")? {
            Some(val) if !val.trim().is_empty() => Ok(Some(val)),
            _ => Ok(None),
        }
    }

    /// Persists the last selected rune key in SQLite.
    pub fn set_selected_rune_key(&self, rune_key: Option<&str>) -> Result<(), String> {
        let val = rune_key.unwrap_or("");
        self.set_setting("selected_rune_key", val)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rune_store_role_persistence() {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let temp_dir = std::env::temp_dir().join(format!("lcuc_test_{}", nanos));
        std::fs::create_dir_all(&temp_dir).unwrap();
        let db_path = temp_dir.join("test.db");

        let store = RuneStore {
            db_path: db_path.clone(),
        };
        store.initialize_schema().expect("Schema init failed");

        let perks = vec![8010, 9111, 9104, 8299, 8126, 8135, 5008, 5008, 5002];
        let id = store
            .save_runeset(
                "Test Mid Rune",
                Some(8000),
                Some(8100),
                &perks,
                None,
                64,
                2, // Mid
                None,
                None,
                "companion_custom",
            )
            .expect("Failed saving runeset");

        let loaded = store
            .get_runeset(id)
            .expect("Query failed")
            .expect("Not found");
        assert_eq!(loaded.role, 2);
        assert_eq!(loaded.champion_id, 64);
        assert_eq!(loaded.rune_page_name, "Test Mid Rune");

        // Test update role
        store
            .update_runeset(
                id,
                "Test Top Rune",
                Some(8000),
                Some(8100),
                &perks,
                Some(64),
                Some(0), // Top
            )
            .expect("Update failed");

        let updated = store
            .get_runeset(id)
            .expect("Query failed")
            .expect("Not found");
        assert_eq!(updated.role, 0);
        assert_eq!(updated.rune_page_name, "Test Top Rune");

        let list = store.list_runesets(10).expect("List failed");
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].role, 0);

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
