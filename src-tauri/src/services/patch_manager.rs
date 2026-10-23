//! # Patch Detection & Cache Invalidation Coordinator
//!
//! Detects the active League of Legends patch version from the local League Client (LCU)
//! or CommunityDragon metadata (`content-metadata.json`), persists the active patch marker
//! to disk, and automatically invalidates all in-memory and on-disk asset/catalog caches
//! whenever a new patch arrives.

use crate::logger;
use crate::services::lcu_client::LcuClient;
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs;
use std::path::PathBuf;
use std::sync::RwLock as SyncRwLock;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::sync::Mutex;

const CDRAGON_METADATA_URL: &str = "https://raw.communitydragon.org/latest/content-metadata.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CacheMetadata {
    patch: String,
    raw_version: Option<String>,
    updated_at: u64,
}

lazy_static! {
    static ref ACTIVE_PATCH_SYNC: SyncRwLock<Option<String>> = SyncRwLock::new(None);
    static ref LAST_PATCH_CHECK: Mutex<Option<Instant>> = Mutex::new(None);
}

fn cache_metadata_path() -> PathBuf {
    dirs::data_local_dir()
        .or_else(dirs::home_dir)
        .unwrap_or_else(|| PathBuf::from("."))
        .join("lcu-companion")
        .join("cache")
        .join("cache_metadata.json")
}

/// Normalizes a raw League version string (e.g. `"16.20.623.6777"` or `"16.20"`)
/// into a canonical `"major.minor"` patch string (e.g. `"16.20"`).
pub fn normalize_patch(raw_version: &str) -> Option<String> {
    let trimmed = raw_version.trim().trim_start_matches(['v', 'V']);
    if trimmed.is_empty() {
        return None;
    }
    let mut parts = trimmed.split(['.', '_', '-', ' ']);
    let major_str = parts.next()?;
    let minor_str = parts.next()?;
    let major: u32 = major_str.parse().ok()?;
    let minor: u32 = minor_str.parse().ok()?;
    if major == 0 {
        return None;
    }
    Some(format!("{major}.{minor}"))
}

/// Parses a normalized `"major.minor"` patch string into `(major, minor)`.
pub fn parse_patch_numbers(patch: &str) -> Option<(u32, u32)> {
    let normalized = normalize_patch(patch)?;
    let mut parts = normalized.split('.');
    let major = parts.next()?.parse::<u32>().ok()?;
    let minor = parts.next()?.parse::<u32>().ok()?;
    Some((major, minor))
}

/// Reads the persisted patch from memory or `cache_metadata.json`.
pub fn get_cached_patch() -> Option<String> {
    if let Ok(guard) = ACTIVE_PATCH_SYNC.read() {
        if let Some(ref patch) = *guard {
            return Some(patch.clone());
        }
    }
    let path = cache_metadata_path();
    let metadata: Option<CacheMetadata> = fs::read(path)
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok());
    if let Some(meta) = metadata {
        if let Some(normalized) = normalize_patch(&meta.patch) {
            if let Ok(mut guard) = ACTIVE_PATCH_SYNC.write() {
                *guard = Some(normalized.clone());
            }
            return Some(normalized);
        }
    }
    None
}

/// Returns the cached `(major, minor)` patch numbers if known.
pub fn get_cached_patch_numbers() -> Option<(u32, u32)> {
    get_cached_patch().and_then(|p| parse_patch_numbers(&p))
}

fn write_cached_patch(patch: &str, raw_version: Option<&str>) {
    if let Ok(mut guard) = ACTIVE_PATCH_SYNC.write() {
        *guard = Some(patch.to_string());
    }
    let path = cache_metadata_path();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let updated_at = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let metadata = CacheMetadata {
        patch: patch.to_string(),
        raw_version: raw_version.map(str::to_string),
        updated_at,
    };
    if let Ok(bytes) = serde_json::to_vec_pretty(&metadata) {
        let _ = fs::write(path, bytes);
    }
}

/// Queries the running League Client or CommunityDragon for the current game version.
pub async fn fetch_live_version(lcu_client: Option<&LcuClient>) -> Option<(String, String)> {
    // 1. Prefer live LCU client version when available
    if let Some(client) = lcu_client {
        if let Ok(ver_val) = client.get_json::<Value>("/lol-patch/v1/game-version").await {
            if let Some(raw) = ver_val.as_str() {
                if let Some(patch) = normalize_patch(raw) {
                    return Some((patch, raw.to_string()));
                }
            }
        }
        if let Ok(builds_val) = client.get_json::<Value>("/system/v1/builds").await {
            if let Some(raw) = builds_val.get("version").and_then(Value::as_str) {
                if let Some(patch) = normalize_patch(raw) {
                    return Some((patch, raw.to_string()));
                }
            }
        }
    }

    // 2. Fallback to CommunityDragon content-metadata.json
    if let Ok(http) = reqwest::Client::builder()
        .timeout(Duration::from_secs(6))
        .build()
    {
        if let Ok(resp) = http.get(CDRAGON_METADATA_URL).send().await {
            if let Ok(json) = resp.json::<Value>().await {
                if let Some(raw) = json.get("version").and_then(Value::as_str) {
                    if let Some(patch) = normalize_patch(raw) {
                        return Some((patch, raw.to_string()));
                    }
                }
            }
        }
    }

    None
}

/// Invalidates all in-memory and disk CDragon caches across the application.
pub async fn invalidate_all_caches() {
    crate::services::asset_cache::invalidate_asset_cache().await;
    crate::services::history::invalidate_history_caches().await;
    crate::services::champions::invalidate_champions_cache().await;
    crate::services::rune_tree::invalidate_rune_tree_cache().await;
}

/// Checks the active game patch and automatically purges/refreshes all caches if the patch changed.
/// Returns `(active_patch, patch_changed)`.
pub async fn ensure_patch_freshness(lcu_client: Option<&LcuClient>) -> (Option<String>, bool) {
    let cached_patch = get_cached_patch();

    // Avoid spamming remote CDragon checks if we already checked recently and have no LCU client
    if lcu_client.is_none() && cached_patch.is_some() {
        let mut last_check = LAST_PATCH_CHECK.lock().await;
        if let Some(instant) = *last_check {
            if instant.elapsed() < Duration::from_secs(300) {
                return (cached_patch, false);
            }
        }
        *last_check = Some(Instant::now());
    }

    let Some((live_patch, raw_version)) = fetch_live_version(lcu_client).await else {
        return (cached_patch, false);
    };

    {
        let mut last_check = LAST_PATCH_CHECK.lock().await;
        *last_check = Some(Instant::now());
    }

    let patch_changed = cached_patch.as_deref() != Some(live_patch.as_str());
    if patch_changed {
        logger::info(
            "tauri.patch_manager",
            &format!(
                "Detected game patch {} (raw: {}, previous: {:?}); invalidating cached assets and catalogs",
                live_patch, raw_version, cached_patch
            ),
        );
        invalidate_all_caches().await;
        write_cached_patch(&live_patch, Some(&raw_version));
    }

    (Some(live_patch), patch_changed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_patch() {
        assert_eq!(
            normalize_patch("16.20.623.6777"),
            Some("16.20".to_string())
        );
        assert_eq!(normalize_patch("v15.8.1"), Some("15.8".to_string()));
        assert_eq!(normalize_patch("16.1"), Some("16.1".to_string()));
        assert_eq!(normalize_patch(""), None);
        assert_eq!(normalize_patch("invalid"), None);
    }
}

