use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

use crate::build_engine::bitset_miner::ChampionTransactionIndex;
use crate::build_engine::data_loader::load_champion_matches;

pub struct CachedIndexEntry {
    pub index: Arc<ChampionTransactionIndex>,
    pub cached_at: Instant,
}

lazy_static::lazy_static! {
    static ref INDEX_CACHE: RwLock<HashMap<u32, CachedIndexEntry>> = RwLock::new(HashMap::new());
}

const CACHE_TTL: Duration = Duration::from_secs(300); // 5 minutes TTL

pub fn get_or_build_champion_index(
    champion_id: u32,
) -> Result<(Arc<ChampionTransactionIndex>, bool), String> {
    // 1. Check read lock for warm cache hit
    {
        let cache = INDEX_CACHE.read().map_err(|e| e.to_string())?;
        if let Some(entry) = cache.get(&champion_id) {
            if entry.cached_at.elapsed() < CACHE_TTL {
                return Ok((Arc::clone(&entry.index), true));
            }
        }
    }

    // 2. Cold load from database and build bitset index
    let matches = load_champion_matches(champion_id)?;
    let index = Arc::new(ChampionTransactionIndex::from_matches(champion_id, &matches));

    // 3. Write to cache
    {
        let mut cache = INDEX_CACHE.write().map_err(|e| e.to_string())?;
        cache.insert(
            champion_id,
            CachedIndexEntry {
                index: Arc::clone(&index),
                cached_at: Instant::now(),
            },
        );
    }

    Ok((index, false))
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
