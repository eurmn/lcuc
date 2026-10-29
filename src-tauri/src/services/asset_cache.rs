//! # CDragon Asset In-Disk Caching Service
//!
//! Provides disk and memory caching for CommunityDragon remote assets
//! (summoner icons, ranked wings, prestige borders, challenge tokens, and rune perks).
//!
//! Converts fetched assets into self-contained base64 data URIs for instantaneous, offline-ready UI rendering.

use base64::prelude::*;
use lazy_static::lazy_static;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::time::Duration;
use tokio::sync::RwLock;

lazy_static! {
    static ref ASSET_MEMORY_CACHE: RwLock<HashMap<String, String>> = RwLock::new(HashMap::new());
}

/// Returns the local base directory path used for file and asset caching.
pub fn get_cache_dir() -> PathBuf {
    dirs::data_local_dir()
        .or_else(dirs::home_dir)
        .unwrap_or_else(|| PathBuf::from("."))
        .join("lcu-companion")
        .join("cache")
}

/// Returns the local directory path used for CDragon in-disk asset caching.
pub fn get_asset_cache_dir() -> PathBuf {
    get_cache_dir().join("cdragon")
}

/// Clears both the in-memory and on-disk CDragon asset caches (triggered on patch updates).
pub async fn invalidate_asset_cache() {
    {
        let mut mem = ASSET_MEMORY_CACHE.write().await;
        mem.clear();
    }
    let cache_dir = get_asset_cache_dir();
    if cache_dir.exists() {
        let _ = fs::remove_dir_all(&cache_dir);
        let _ = fs::create_dir_all(&cache_dir);
    }
}

/// Converts a remote asset URL into a safe, deterministic filename on disk.
fn url_to_filename(url: &str) -> String {
    let ext = if url.ends_with(".jpg") || url.ends_with(".jpeg") {
        "jpg"
    } else if url.ends_with(".svg") {
        "svg"
    } else if url.ends_with(".webp") {
        "webp"
    } else {
        "png"
    };

    let sanitized: String = url
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect();

    // Use truncated sanitized string + simple checksum for uniqueness
    let hash: u64 = url
        .bytes()
        .fold(0u64, |acc, b| acc.wrapping_mul(31).wrapping_add(b as u64));
    format!(
        "{}_{:x}.{}",
        &sanitized[..sanitized.len().min(40)],
        hash,
        ext
    )
}

/// Determines the MIME type string from a filename or URL.
fn get_mime_type(url: &str) -> &'static str {
    if url.ends_with(".jpg") || url.ends_with(".jpeg") {
        "image/jpeg"
    } else if url.ends_with(".svg") {
        "image/svg+xml"
    } else if url.ends_with(".webp") {
        "image/webp"
    } else {
        "image/png"
    }
}

/// Fetches a remote CDragon asset, caches it on disk, and returns a client-ready base64 Data URI.
///
/// If already cached in memory or on disk, no network request is performed.
pub async fn get_cached_asset_data_uri(remote_url: &str) -> String {
    let url = remote_url.trim();
    if url.is_empty() || url.starts_with("data:") {
        return url.to_string();
    }

    // 1. Check in-memory cache
    {
        let mem = ASSET_MEMORY_CACHE.read().await;
        if let Some(data_uri) = mem.get(url) {
            return data_uri.clone();
        }
    }

    // 2. Check disk cache
    let cache_dir = get_asset_cache_dir();
    let filename = url_to_filename(url);
    let disk_path = cache_dir.join(&filename);

    let bytes = if disk_path.exists()
        && fs::metadata(&disk_path)
            .map(|m| m.len() > 0)
            .unwrap_or(false)
    {
        fs::read(&disk_path).ok()
    } else {
        None
    };

    let final_bytes = match bytes {
        Some(b) => b,
        None => {
            // 3. Fetch from remote server
            let client = reqwest::Client::builder()
                .timeout(Duration::from_secs(8))
                .build()
                .unwrap_or_default();

            match client.get(url).send().await {
                Ok(resp) if resp.status().is_success() => match resp.bytes().await {
                    Ok(b) => {
                        let vec_bytes = b.to_vec();
                        // Write to disk cache
                        let _ = fs::create_dir_all(&cache_dir);
                        let _ = fs::write(&disk_path, &vec_bytes);
                        vec_bytes
                    }
                    Err(_) => return url.to_string(),
                },
                _ => return url.to_string(),
            }
        }
    };

    let mime = get_mime_type(url);
    let data_uri = format!(
        "data:{};base64,{}",
        mime,
        BASE64_STANDARD.encode(&final_bytes)
    );

    // 4. Save into in-memory cache
    {
        let mut mem = ASSET_MEMORY_CACHE.write().await;
        mem.insert(url.to_string(), data_uri.clone());
    }

    data_uri
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_url_to_filename() {
        let name = url_to_filename("https://raw.communitydragon.org/latest/icon.png");
        assert!(name.ends_with(".png"));
    }

    #[test]
    fn test_mime_type() {
        assert_eq!(get_mime_type("test.jpg"), "image/jpeg");
        assert_eq!(get_mime_type("test.png"), "image/png");
    }
}
