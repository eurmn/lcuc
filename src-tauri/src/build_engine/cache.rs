use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

use crate::build_engine::bitset_miner::ChampionTransactionIndex;
use crate::build_engine::data_loader::load_champion_matches;
use crate::build_engine::models::BuildRecommendationResult;

pub struct CachedIndexEntry {
    pub index: Arc<ChampionTransactionIndex>,
    pub cached_at: Instant,
    pub access_count: usize,
}

pub struct CachedResultEntry {
    pub result: BuildRecommendationResult,
    pub cached_at: Instant,
    pub champion_id: u32,
}

const INDEX_CACHE_TTL: Duration = Duration::from_secs(300); // 5 minutes TTL
const RESULT_CACHE_TTL: Duration = Duration::from_secs(300); // 5 minutes TTL
const MAX_CACHE_ENTRIES: usize = 128; // bounded LRU capacity

lazy_static::lazy_static! {
    /// ChampionTransactionIndex cache: keyed by champion_id only (index is champion-specific).
    static ref INDEX_CACHE: RwLock<HashMap<u32, CachedIndexEntry>> = RwLock::new(HashMap::new());

    /// Recommendation result cache: keyed by hash of (champion_id, sorted_items, sorted_augments)
    /// to skip full recomputation on identical queries.
    static ref RESULT_CACHE: RwLock<HashMap<u64, CachedResultEntry>> = RwLock::new(HashMap::new());
}

/// Compute a deterministic cache key from champion_id + sorted owned items + sorted owned augments.
pub fn compute_recommendation_cache_key(
    champion_id: u32,
    owned_items: &[u32],
    owned_augments: &[String],
) -> u64 {
    let mut items: Vec<u32> = owned_items.to_vec();
    items.sort_unstable();
    let mut augs: Vec<String> = owned_augments.to_vec();
    augs.sort_unstable();

    let mut hash: u64 = champion_id as u64;
    for &id in &items {
        hash = hash.wrapping_mul(31).wrapping_add(id as u64);
    }
    for aug in &augs {
        for b in aug.bytes() {
            hash = hash.wrapping_mul(31).wrapping_add(b as u64);
        }
    }
    hash
}

fn evict_if_over_capacity<K, V>(cache: &mut HashMap<K, V>)
where
    K: Clone + Eq + std::hash::Hash,
{
    if cache.len() > MAX_CACHE_ENTRIES {
        // Simple LRU-ish eviction: remove the entry with the lowest access_count (or oldest cached_at).
        // For simplicity, evict the first N entries (arbitrary but bounds growth).
        let to_remove = cache.len() - MAX_CACHE_ENTRIES;
        let keys: Vec<K> = cache.keys().cloned().take(to_remove).collect();
        for k in keys {
            cache.remove(&k);
        }
    }
}

pub fn get_or_build_champion_index(
    champion_id: u32,
) -> Result<(Arc<ChampionTransactionIndex>, bool), String> {
    // 1. Check read lock for warm cache hit
    {
        let cache = INDEX_CACHE.read().map_err(|e| e.to_string())?;
        if let Some(entry) = cache.get(&champion_id) {
            if entry.cached_at.elapsed() < INDEX_CACHE_TTL {
                return Ok((Arc::clone(&entry.index), true));
            }
        }
    }

    // 2. Cold load from database and build bitset index
    let matches = load_champion_matches(champion_id)?;
    let index = Arc::new(ChampionTransactionIndex::from_matches(champion_id, &matches));

    // 3. Write to cache (bounded)
    {
        let mut cache = INDEX_CACHE.write().map_err(|e| e.to_string())?;
        cache.insert(
            champion_id,
            CachedIndexEntry {
                index: Arc::clone(&index),
                cached_at: Instant::now(),
                access_count: 1,
            },
        );
        evict_if_over_capacity(&mut cache);
    }

    Ok((index, false))
}

/// Get a cached build recommendation result if present and not expired.
pub fn get_cached_result(cache_key: u64) -> Option<BuildRecommendationResult> {
    let cache = RESULT_CACHE.read().ok()?;
    let entry = cache.get(&cache_key)?;
    if entry.cached_at.elapsed() >= RESULT_CACHE_TTL {
        return None;
    }
    Some(entry.result.clone())
}

/// Store a build recommendation result in the cache.
pub fn cache_result(champion_id: u32, cache_key: u64, result: BuildRecommendationResult) {
    if let Ok(mut cache) = RESULT_CACHE.write() {
        cache.insert(
            cache_key,
            CachedResultEntry {
                result,
                cached_at: Instant::now(),
                champion_id,
            },
        );
        evict_if_over_capacity(&mut cache);
    }
}

pub fn invalidate_champion_cache(champion_id: Option<u32>) {
    if let Ok(mut cache) = INDEX_CACHE.write() {
        if let Some(cid) = champion_id {
            cache.remove(&cid);
        } else {
            cache.clear();
        }
    }
}

/// Clear the result cache (optionally for a specific champion).
pub fn invalidate_result_cache(champion_id: Option<u32>) {
    if let Ok(mut cache) = RESULT_CACHE.write() {
        if let Some(cid) = champion_id {
            cache.retain(|_, entry| entry.champion_id != cid);
        } else {
            cache.clear();
        }
    }
}
