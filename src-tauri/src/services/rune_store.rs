//! # Rune Store SQLite Repository
//!
//! Provides local database persistence for custom and imported rune pages
//! using embedded SQLite via the `rusqlite` crate.

use crate::models::{RuneSetRecord, RuneSetSummary};
use rusqlite::{params, Connection};
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
        fs::create_dir_all(&db_dir)
            .map_err(|e| format!("Failed creating db directory: {}", e))?;

        let mut db_path = db_dir;
        db_path.push("lcu_companion.db");

        let store = Self { db_path };
        store.initialize_schema()?;
        Ok(store)
    }

    /// Opens a connection to the SQLite database.
    fn connect(&self) -> Result<Connection, String> {
        Connection::open(&self.db_path)
            .map_err(|e| format!("Failed connecting to SQLite at {:?}: {}", self.db_path, e))
    }

    /// Initializes the `runesets` table schema if it does not already exist.
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
                created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            )",
            [],
        )
        .map_err(|e| format!("Failed creating runesets schema: {}", e))?;
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
                source
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
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
            let selected_perks: Vec<i64> =
                serde_json::from_str(&perks_json).unwrap_or_default();

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
                created_at: row.get(10).unwrap_or_default(),
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
                    source: row.get(5)?,
                    created_at: row.get(6)?,
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
                    selected_perk_ids_json = ?4
                WHERE id = ?5",
                params![
                    rune_page_name,
                    primary_style_id,
                    sub_style_id,
                    perks_json,
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
}
