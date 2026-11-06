#![allow(non_snake_case)]

use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::{Arc, LazyLock, RwLock};
use std::time::Instant;

use crate::build_engine::item_catalog::{
    get_categories, get_gold_cost, is_completed_boot, is_legendary_item, is_valid_build_item,
};
use crate::logger;

// --- Constants & Key Packing ---

pub const PORO_SNAX_IDS: [u32; 3] = [2050, 2052, 220013];

pub const KIND_CHAMP_BASE: u8 = 0;
pub const KIND_AUG_SINGLE: u8 = 1;
pub const KIND_ITEM_SINGLE: u8 = 2;
pub const KIND_AUG_CHAMP: u8 = 3;
pub const KIND_ITEM_CHAMP: u8 = 4;
pub const KIND_AUG_AUG_GLOBAL: u8 = 5;
pub const KIND_ITEM_ITEM_GLOBAL: u8 = 6;
pub const KIND_ITEM_AUG_GLOBAL: u8 = 7;
pub const KIND_AUG_AUG_CHAMP: u8 = 8;
pub const KIND_ITEM_ITEM_CHAMP: u8 = 9;
pub const KIND_ITEM_AUG_CHAMP: u8 = 10;

#[inline(always)]
pub const fn pack_combo_key(kind: u8, champ_id: u16, entity_a: u32, entity_b: u32) -> u64 {
    (((kind as u64) & 0xF) << 60)
        | (((champ_id as u64) & 0xFFFF) << 44)
        | (((entity_a as u64) & 0x3FFFFF) << 22)
        | ((entity_b as u64) & 0x3FFFFF)
}

#[inline(always)]
pub const fn unpack_combo_key(key: u64) -> (u8, u16, u32, u32) {
    (
        ((key >> 60) & 0xF) as u8,
        ((key >> 44) & 0xFFFF) as u16,
        ((key >> 22) & 0x3FFFFF) as u32,
        (key & 0x3FFFFF) as u32,
    )
}

pub fn parse_or_hash_augment_id(s: &str) -> u32 {
    let trimmed = s.trim();
    if let Ok(id) = trimmed.parse::<u32>() {
        id & 0x3FFFFF
    } else {
        // Fold FNV-1a 64-bit hash into 22 bits
        let mut hash: u64 = 0xcbf29ce484222325;
        for byte in trimmed.as_bytes() {
            hash ^= *byte as u64;
            hash = hash.wrapping_mul(0x100000001b3);
        }
        let folded = ((hash ^ (hash >> 22) ^ (hash >> 44)) & 0x3FFFFF) as u32;
        if folded == 0 {
            1
        } else {
            folded
        }
    }
}

// --- Statistical Math Functions ---

#[inline]
pub fn wilson_lcb(w_tilde: f64, n_tilde: f64, z: f64) -> f64 {
    if n_tilde <= 0.0 {
        return 0.50;
    }
    let p = (w_tilde / n_tilde).clamp(0.0001, 0.9999);
    let z2 = z * z;
    let num = p + z2 / (2.0 * n_tilde)
        - z * ((p * (1.0 - p) + z2 / (4.0 * n_tilde)) / n_tilde).sqrt();
    let res = num / (1.0 + z2 / n_tilde);
    if res.is_finite() {
        res.clamp(0.01, 0.99)
    } else {
        0.50
    }
}

#[inline]
pub fn logit(p: f64) -> f64 {
    let p_c = p.clamp(0.05, 0.95);
    (p_c / (1.0 - p_c)).ln()
}

#[inline]
pub fn sigmoid(x: f64) -> f64 {
    if x > 30.0 {
        0.9999
    } else if x < -30.0 {
        0.0001
    } else {
        1.0 / (1.0 + (-x).exp())
    }
}

pub fn compute_confidence_grade(score: f64) -> &'static str {
    if score >= 0.08 {
        "S+"
    } else if score >= 0.05 {
        "S"
    } else if score >= 0.025 {
        "A"
    } else if score >= 0.0 {
        "B"
    } else if score >= -0.025 {
        "C"
    } else {
        "D"
    }
}

// --- Catalog & Metadata ---

const BUNDLED_AUGMENT_TOOLTIPS_JSON: &str = include_str!("../data/augment_tooltips.json");

#[derive(Debug, Clone)]
pub struct AugmentMeta {
    pub id: String,
    pub numeric_id: u32,
    pub name: String,
    pub tier: String, // "Silver", "Gold", "Prismatic"
    pub tooltip: String,
    pub icon_url: String,
}

pub static AUGMENT_METADATA: LazyLock<HashMap<String, AugmentMeta>> = LazyLock::new(|| {
    let mut map = HashMap::new();
    if let Ok(val) = serde_json::from_str::<serde_json::Value>(BUNDLED_AUGMENT_TOOLTIPS_JSON) {
        if let Some(obj) = val.as_object() {
            for (key, item) in obj {
                let name = item
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let raw_rarity = item
                    .get("rarity")
                    .and_then(|v| v.as_str())
                    .unwrap_or("kSilver");
                let tier = match raw_rarity {
                    "kPrismatic" | "prismatic" | "Prismatic" => "Prismatic".to_string(),
                    "kGold" | "gold" | "Gold" => "Gold".to_string(),
                    _ => "Silver".to_string(),
                };
                let tooltip = item
                    .get("tooltip")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let numeric_id = parse_or_hash_augment_id(key);
                let icon_url = format!(
                    "https://raw.communitydragon.org/latest/plugins/rcp-be-lol-game-data/global/default/assets/perks/champion-augments/{}.png",
                    key
                ).to_lowercase();

                let meta = AugmentMeta {
                    id: key.clone(),
                    numeric_id,
                    name: name.clone(),
                    tier,
                    tooltip,
                    icon_url,
                };
                map.insert(key.clone(), meta.clone());
                map.insert(name.to_lowercase(), meta.clone());
                map.insert(numeric_id.to_string(), meta);
            }
        }
    }
    map
});

pub fn get_augment_meta(id_or_name: &str) -> AugmentMeta {
    if let Some(m) = AUGMENT_METADATA.get(id_or_name) {
        return m.clone();
    }
    let lower = id_or_name.to_lowercase();
    if let Some(m) = AUGMENT_METADATA.get(&lower) {
        return m.clone();
    }
    let num = parse_or_hash_augment_id(id_or_name);
    AugmentMeta {
        id: id_or_name.to_string(),
        numeric_id: num,
        name: if id_or_name.chars().all(|c| c.is_ascii_digit()) {
            format!("Augment #{id_or_name}")
        } else {
            id_or_name.to_string()
        },
        tier: "Gold".to_string(),
        tooltip: String::new(),
        icon_url: format!(
            "https://raw.communitydragon.org/latest/plugins/rcp-be-lol-game-data/global/default/assets/perks/champion-augments/{}.png",
            num
        ).to_lowercase(),
    }
}

// --- In-Memory Synergy Index ---

#[derive(Debug, Clone, Copy, Default)]
pub struct WinGameCount {
    pub wins: u32,
    pub games: u32,
}

#[derive(Debug, Clone, Default)]
pub struct MayhemSynergyIndex {
    pub counts: HashMap<u64, WinGameCount>,
    pub personal_counts: HashMap<u64, WinGameCount>,
    pub tier_counts: HashMap<String, WinGameCount>,
    pub id_to_augment_string: HashMap<u32, String>,
    pub total_matches: u32,
    pub active_patch: Option<String>,
}

lazy_static::lazy_static! {
    pub static ref MAYHEM_SYNERGY_INDEX: Arc<RwLock<Option<MayhemSynergyIndex>>> = Arc::new(RwLock::new(None));
}

pub fn invalidate_synergy_index() {
    if let Ok(mut lock) = MAYHEM_SYNERGY_INDEX.write() {
        *lock = None;
    }
}

// --- Single-Pass N-Gram Miner ---

pub fn build_synergy_index(
    global_db_path: &Path,
    agent_db_path: &Path,
) -> Result<MayhemSynergyIndex, String> {
    let mut index = MayhemSynergyIndex::default();
    let started = Instant::now();

    // 1. Ingest Global ARAM matches (aram_participants)
    if global_db_path.exists() {
        if let Ok(conn) = rusqlite::Connection::open_with_flags(
            global_db_path,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
        ) {
            let _ = conn.busy_timeout(std::time::Duration::from_millis(5000));
            ingest_table_into_index(&conn, "aram_participants", false, &mut index);
        }
    }

    // 2. Ingest Personal facts (game_build_facts)
    if agent_db_path.exists() {
        if let Ok(conn) = rusqlite::Connection::open_with_flags(
            agent_db_path,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
        ) {
            let _ = conn.busy_timeout(std::time::Duration::from_millis(5000));
            ingest_table_into_index(&conn, "game_build_facts", true, &mut index);
        }
    }

    logger::info(
        "mayhem_engine",
        &format!(
            "Compiled MayhemSynergyIndex: {} keys, {} total matches in {}ms",
            index.counts.len(),
            index.total_matches,
            started.elapsed().as_millis()
        ),
    );

    Ok(index)
}

fn ingest_table_into_index(
    conn: &Connection,
    table: &str,
    is_personal: bool,
    index: &mut MayhemSynergyIndex,
) {
    let query = format!(
        "SELECT champion_id, win, item_ids_json, augment_ids_json FROM {} ORDER BY rowid DESC LIMIT 100000",
        table
    );
    let mut stmt = match conn.prepare(&query) {
        Ok(s) => s,
        Err(_) => return,
    };

    let rows = stmt.query_map([], |row| {
        let champ_id: Option<i64> = row.get(0).ok();
        let win: i64 = row.get(1).unwrap_or(0);
        let items_raw: String = row.get(2).unwrap_or_else(|_| "[]".to_string());
        let augs_raw: String = row.get(3).unwrap_or_else(|_| "[]".to_string());
        Ok((champ_id.unwrap_or(0) as u16, win != 0, items_raw, augs_raw))
    });

    let Ok(rows_iter) = rows else { return };

    for row_res in rows_iter {
        let (champ_id, win, items_raw, augs_raw) = match row_res {
            Ok(r) => r,
            Err(_) => continue,
        };

        if !is_personal {
            index.total_matches += 1;
        }

        // Deduplicate and filter completed items (excluding 2050, 2052, 220013)
        let mut items = Vec::new();
        if let Ok(parsed_items) = serde_json::from_str::<Vec<serde_json::Value>>(&items_raw) {
            for v in parsed_items {
                let id_opt = v.as_u64().map(|n| n as u32).or_else(|| {
                    v.as_str().and_then(|s| s.parse::<u32>().ok())
                });
                if let Some(id) = id_opt {
                    if !PORO_SNAX_IDS.contains(&id)
                        && (is_valid_build_item(id) || is_completed_boot(id))
                        && !items.contains(&id)
                    {
                        items.push(id);
                    }
                }
            }
        }
        items.sort_unstable();
        if items.len() > 6 {
            items.truncate(6);
        }

        // Deduplicate augments (k <= 5)
        let mut augments = Vec::new();
        if let Ok(parsed_augs) = serde_json::from_str::<Vec<serde_json::Value>>(&augs_raw) {
            for v in parsed_augs {
                let s_opt = v.as_str().map(|s| s.to_string()).or_else(|| v.as_u64().map(|n| n.to_string()));
                if let Some(s) = s_opt {
                    let num = parse_or_hash_augment_id(&s);
                    index.id_to_augment_string.entry(num).or_insert_with(|| s.clone());
                    if !augments.contains(&num) {
                        augments.push(num);
                        let meta = get_augment_meta(&s);
                        let tier_entry = index.tier_counts.entry(meta.tier).or_default();
                        tier_entry.games += 1;
                        if win {
                            tier_entry.wins += 1;
                        }
                    }
                }
            }
        }
        augments.sort_unstable();
        if augments.len() > 5 {
            augments.truncate(5);
        }

        let emit = |key: u64, is_win: bool, idx: &mut MayhemSynergyIndex| {
            let map = if is_personal {
                &mut idx.personal_counts
            } else {
                &mut idx.counts
            };
            let entry = map.entry(key).or_default();
            entry.games += 1;
            if is_win {
                entry.wins += 1;
            }
        };

        // Kind 0: ChampBase
        if champ_id > 0 {
            emit(pack_combo_key(KIND_CHAMP_BASE, champ_id, 0, 0), win, index);
        }

        // Kind 1 (AugSingle) & Kind 3 (AugChamp)
        for &a in &augments {
            emit(pack_combo_key(KIND_AUG_SINGLE, 0, a, 0), win, index);
            if champ_id > 0 {
                emit(pack_combo_key(KIND_AUG_CHAMP, champ_id, a, 0), win, index);
            }
        }

        // Kind 2 (ItemSingle) & Kind 4 (ItemChamp)
        for &i in &items {
            emit(pack_combo_key(KIND_ITEM_SINGLE, 0, i, 0), win, index);
            if champ_id > 0 {
                emit(pack_combo_key(KIND_ITEM_CHAMP, champ_id, i, 0), win, index);
            }
        }

        // Kind 5 (AugAugGlobal) & Kind 8 (AugAugChamp)
        for x in 0..augments.len() {
            for y in (x + 1)..augments.len() {
                let a1 = augments[x].min(augments[y]);
                let a2 = augments[x].max(augments[y]);
                emit(pack_combo_key(KIND_AUG_AUG_GLOBAL, 0, a1, a2), win, index);
                if champ_id > 0 {
                    emit(pack_combo_key(KIND_AUG_AUG_CHAMP, champ_id, a1, a2), win, index);
                }
            }
        }

        // Kind 6 (ItemItemGlobal) & Kind 9 (ItemItemChamp)
        for x in 0..items.len() {
            for y in (x + 1)..items.len() {
                let i1 = items[x].min(items[y]);
                let i2 = items[x].max(items[y]);
                emit(pack_combo_key(KIND_ITEM_ITEM_GLOBAL, 0, i1, i2), win, index);
                if champ_id > 0 {
                    emit(pack_combo_key(KIND_ITEM_ITEM_CHAMP, champ_id, i1, i2), win, index);
                }
            }
        }

        // Kind 7 (ItemAugGlobal) & Kind 10 (ItemAugChamp) - always a = aug_id, b = item_id
        for &a in &augments {
            for &i in &items {
                emit(pack_combo_key(KIND_ITEM_AUG_GLOBAL, 0, a, i), win, index);
                if champ_id > 0 {
                    emit(pack_combo_key(KIND_ITEM_AUG_CHAMP, champ_id, a, i), win, index);
                }
            }
        }
    }
}

// --- DTO Contracts ---

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MayhemBuildRecommendationRequest {
    #[serde(alias = "champion_id")]
    pub champion_id: u32,
    #[serde(default, alias = "locked_augments", alias = "owned_augments")]
    pub locked_augments: Vec<String>,
    #[serde(default, alias = "locked_items", alias = "owned_items")]
    pub locked_items: Vec<u32>,
    #[serde(default, alias = "offered_cards")]
    pub offered_cards: Option<Vec<String>>,
    #[serde(default, alias = "seen_augments")]
    pub seen_augments: Option<Vec<String>>,
    #[serde(default, alias = "augment_slot")]
    pub augment_slot: Option<u8>,
    #[serde(default, alias = "command_line_reroll")]
    pub command_line_reroll: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivePairSynergyDto {
    pub combo_type: String,
    pub item_ids: Vec<u32>,
    pub augment_ids: Vec<String>,
    pub label: String,
    pub win_rate: f64,
    pub wpa: f64,
    pub synergy_lift: f64,
    pub lcb_synergy_lift: f64,
    pub sample_size: usize,
    pub confidence_grade: String,

    // camelCase mirrors
    pub comboType: String,
    pub itemIds: Vec<u32>,
    pub augmentIds: Vec<String>,
    pub winRate: f64,
    pub synergyLift: f64,
    pub lcbSynergyLift: f64,
    pub sampleSize: usize,
    pub confidenceGrade: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AugmentCardEvalDto {
    pub augment_id: String,
    pub name: String,
    pub icon_url: String,
    pub tier: String,
    pub tooltip: String,
    pub score: f64,
    pub wpa: f64,
    pub kit_lift: f64,
    pub lcb_wpa: f64,
    pub sample_size: usize,
    pub verdict: String, // "best_pick", "reroll", "neutral"
    pub confidence_grade: String,
    pub expected_reroll_gain: Option<f64>,
    pub upgrade_probability: Option<f64>,
    pub active_combos: Vec<ActivePairSynergyDto>,
    pub lookahead_partner_id: Option<u32>,
    pub lookahead_partner_kind: Option<String>,
    pub lookahead_synergy_lift: Option<f64>,

    // camelCase mirrors
    pub augmentId: String,
    pub iconUrl: String,
    pub kitLift: f64,
    pub lcbWpa: f64,
    pub sampleSize: usize,
    pub confidenceGrade: String,
    pub expectedRerollGain: Option<f64>,
    pub upgradeProbability: Option<f64>,
    pub activeCombos: Vec<ActivePairSynergyDto>,
    pub lookaheadPartnerId: Option<u32>,
    pub lookaheadPartnerKind: Option<String>,
    pub lookaheadSynergyLift: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RerollEvDto {
    pub ev_reroll: f64,
    pub pool_size: usize,
    pub tier: String,
    pub target_tier: String,
    pub best_pick_id: Option<String>,
    pub should_reroll: bool,

    // camelCase mirrors
    pub evReroll: f64,
    pub poolSize: usize,
    pub targetTier: String,
    pub bestPickId: Option<String>,
    pub shouldReroll: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AugmentRecommendationDto {
    pub augment_id: String,
    pub name: String,
    pub icon_url: String,
    pub tier: String,
    pub tooltip: String,
    pub score: f64,
    pub wpa: f64,
    pub kit_lift: f64,
    pub lcb_wpa: f64,
    pub sample_size: usize,
    pub confidence_grade: String,
    pub lookahead_partner_id: Option<u32>,
    pub lookahead_partner_kind: Option<String>,
    pub lookahead_synergy_lift: Option<f64>,

    // camelCase mirrors
    pub augmentId: String,
    pub iconUrl: String,
    pub kitLift: f64,
    pub lcbWpa: f64,
    pub sampleSize: usize,
    pub confidenceGrade: String,
    pub lookaheadPartnerId: Option<u32>,
    pub lookaheadPartnerKind: Option<String>,
    pub lookaheadSynergyLift: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ItemRecommendationDto {
    pub item_id: u32,
    pub win_rate: f64,
    pub lift: f64,
    pub sample_size: usize,
    pub is_boot: bool,
    pub reason: String,
    pub synergy_lift: f64,
    pub lcb_lift: f64,
    pub confidence_grade: String,

    // camelCase mirrors
    pub itemId: u32,
    pub winRate: f64,
    pub sampleSize: usize,
    pub isBoot: bool,
    pub synergyLift: f64,
    pub lcbLift: f64,
    pub confidenceGrade: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct EnginePerformanceStatsDto {
    pub load_time_ms: f64,
    pub compute_time_ms: f64,
    pub total_time_ms: f64,
    pub match_count: usize,
    pub cache_hit: bool,

    // camelCase mirrors
    pub loadTimeMs: f64,
    pub computeTimeMs: f64,
    pub totalTimeMs: f64,
    pub matchCount: usize,
    pub cacheHit: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MayhemBuildRecommendationResponse {
    pub champion_id: u32,
    pub champion_baseline_winrate: f64,
    pub build_winrate: f64,
    pub build_wpa: f64,
    pub active_combos: Vec<ActivePairSynergyDto>,
    pub anti_synergies: Vec<ActivePairSynergyDto>,
    pub evaluated_cards: Vec<AugmentCardEvalDto>,
    pub reroll_ev: Option<RerollEvDto>,
    pub next_items: Vec<ItemRecommendationDto>,
    pub recommended_boots: Option<ItemRecommendationDto>,
    pub completed_build: Vec<ItemRecommendationDto>,
    pub alternative_items: Vec<ItemRecommendationDto>,
    pub recommended_augments_by_tier: HashMap<String, Vec<AugmentRecommendationDto>>,
    pub stats: EnginePerformanceStatsDto,

    // camelCase mirrors
    pub championId: u32,
    pub championBaselineWinrate: f64,
    pub buildWinrate: f64,
    pub buildWpa: f64,
    pub activeCombos: Vec<ActivePairSynergyDto>,
    pub antiSynergies: Vec<ActivePairSynergyDto>,
    pub evaluatedCards: Vec<AugmentCardEvalDto>,
    pub rerollEv: Option<RerollEvDto>,
    pub nextItems: Vec<ItemRecommendationDto>,
    pub recommendedBoots: Option<ItemRecommendationDto>,
    pub completedBuild: Vec<ItemRecommendationDto>,
    pub alternativeItems: Vec<ItemRecommendationDto>,
    pub recommendedAugmentsByTier: HashMap<String, Vec<AugmentRecommendationDto>>,
}

// --- Query Combos DTO ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComboComponentDto {
    pub kind: String, // "champion", "augment", "item"
    pub id: serde_json::Value,
    pub name: String,
    pub icon_url: String,
    pub rarity: Option<String>,
    pub item_class: Option<String>,
    pub description: Option<String>,
    pub price_total: Option<u32>,
    pub categories: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MayhemComboDto {
    pub id: String,
    pub name: String,
    pub category: String,
    pub champion_id: u16,
    pub components: Vec<ComboComponentDto>,
    pub games: u32,
    pub wins: u32,
    pub win_rate: f64,
    pub combo_wpa: f64,
    pub wpa: f64,
    pub synergy_lift: f64,
    pub lift: f64,
    pub synergy_score: f64,
    pub lcb_synergy_lift: f64,
    pub confidence_grade: String,

    // camelCase mirrors
    pub championId: u16,
    pub winRate: f64,
    pub comboWpa: f64,
    pub synergyLift: f64,
    pub synergyScore: f64,
    pub lcbSynergyLift: f64,
    pub confidenceGrade: String,
}

// --- 4-Level Hierarchical Bayesian Engine Computation ---

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct EvaluatedResource {
    pub id: u32,
    pub is_augment: bool,
    pub p_0: f64,
    pub p_glob: f64,
    pub p_blend: f64,
    pub wpa: f64,
    pub wpa_glob: f64,
    pub delta_champ: f64,
    pub lcb_wpa: f64,
    pub games_glob: u32,
    pub games_pers: u32,
}

impl MayhemSynergyIndex {
    pub fn get_counts(&self, key: u64) -> WinGameCount {
        self.counts.get(&key).copied().unwrap_or_default()
    }

    pub fn get_personal_counts(&self, key: u64) -> WinGameCount {
        self.personal_counts.get(&key).copied().unwrap_or_default()
    }

    pub fn get_tier_winrate(&self, tier: &str) -> f64 {
        if let Some(c) = self.tier_counts.get(tier) {
            if c.games > 0 {
                return (c.wins as f64 / c.games as f64).clamp(0.05, 0.95);
            }
        }
        0.50
    }

    pub fn evaluate_champion_baseline(&self, champ_id: u16) -> f64 {
        let base = self.get_counts(pack_combo_key(KIND_CHAMP_BASE, champ_id, 0, 0));
        let w_c = base.wins as f64;
        let n_c = base.games as f64;
        let m_c = 30.0;
        ((w_c + m_c * 0.50) / (n_c + m_c)).clamp(0.05, 0.95)
    }

    pub fn evaluate_resource(
        &self,
        champ_id: u16,
        resource_id: u32,
        is_augment: bool,
        p_champ: f64,
    ) -> EvaluatedResource {
        // Survivorship offset
        let (delta_tier, p_bar_tier) = if is_augment {
            let aug_str = self
                .id_to_augment_string
                .get(&resource_id)
                .map(String::as_str)
                .unwrap_or("");
            let meta = get_augment_meta(aug_str);
            let p_bar = self.get_tier_winrate(&meta.tier);
            (p_bar - 0.50, p_bar)
        } else {
            (0.0, 0.50)
        };

        let p_0 = (p_champ + delta_tier).clamp(0.05, 0.95);

        let (kind_single, kind_champ) = if is_augment {
            (KIND_AUG_SINGLE, KIND_AUG_CHAMP)
        } else {
            (KIND_ITEM_SINGLE, KIND_ITEM_CHAMP)
        };

        let glob_single = self.get_counts(pack_combo_key(kind_single, 0, resource_id, 0));
        let glob_champ = self.get_counts(pack_combo_key(kind_champ, champ_id, resource_id, 0));
        let pers_champ = self.get_personal_counts(pack_combo_key(kind_champ, champ_id, resource_id, 0));

        let m_glob = 50.0;
        let p_glob_resource = ((glob_single.wins as f64 + m_glob * p_bar_tier)
            / (glob_single.games as f64 + m_glob))
            .clamp(0.05, 0.95);
        let wpa_glob = p_glob_resource - p_bar_tier;

        let m_1 = 25.0;
        let w_rc_glob = glob_champ.wins as f64;
        let n_rc_glob = glob_champ.games as f64;
        let p_glob = ((w_rc_glob + m_1 * p_0) / (n_rc_glob + m_1)).clamp(0.05, 0.95);

        let m_pers = 10.0;
        let w_rc_pers = pers_champ.wins as f64;
        let n_rc_pers = pers_champ.games as f64;
        let p_blend = ((w_rc_pers + m_pers * p_glob) / (n_rc_pers + m_pers)).clamp(0.05, 0.95);

        let wpa = p_blend - p_0;
        let delta_champ = wpa - wpa_glob;

        let w_tilde = w_rc_glob + m_1 * p_0;
        let n_tilde = n_rc_glob + m_1;
        let lcb_val = wilson_lcb(w_tilde, n_tilde, 1.28155);
        let lcb_wpa = lcb_val - p_0;

        EvaluatedResource {
            id: resource_id,
            is_augment,
            p_0,
            p_glob,
            p_blend,
            wpa,
            wpa_glob,
            delta_champ,
            lcb_wpa,
            games_glob: glob_champ.games,
            games_pers: pers_champ.games,
        }
    }

    pub fn evaluate_pair_synergy(
        &self,
        champ_id: u16,
        r1: &EvaluatedResource,
        r2: &EvaluatedResource,
        p_champ: f64,
    ) -> (f64, f64, f64, usize) {
        // Additive independence expectation
        let p_indep = (p_champ + r1.wpa + r2.wpa).clamp(0.05, 0.95);
        let p_indep_glob = (0.50 + r1.wpa_glob + r2.wpa_glob).clamp(0.05, 0.95);

        let (kind_glob, kind_champ, ent_a, ent_b) = match (r1.is_augment, r2.is_augment) {
            (true, true) => {
                let a1 = r1.id.min(r2.id);
                let a2 = r1.id.max(r2.id);
                (KIND_AUG_AUG_GLOBAL, KIND_AUG_AUG_CHAMP, a1, a2)
            }
            (false, false) => {
                let i1 = r1.id.min(r2.id);
                let i2 = r1.id.max(r2.id);
                (KIND_ITEM_ITEM_GLOBAL, KIND_ITEM_ITEM_CHAMP, i1, i2)
            }
            (true, false) => (KIND_ITEM_AUG_GLOBAL, KIND_ITEM_AUG_CHAMP, r1.id, r2.id),
            (false, true) => (KIND_ITEM_AUG_GLOBAL, KIND_ITEM_AUG_CHAMP, r2.id, r1.id),
        };

        let pair_glob = self.get_counts(pack_combo_key(kind_glob, 0, ent_a, ent_b));
        let m_pair = 20.0;
        let p_pair_glob = ((pair_glob.wins as f64 + m_pair * p_indep_glob)
            / (pair_glob.games as f64 + m_pair))
            .clamp(0.05, 0.95);
        let delta_syn_glob = p_pair_glob - p_indep_glob;

        let elig = if r1.games_glob > 0 && r2.games_glob > 0 {
            1.0
        } else {
            0.0
        };

        let p_prior_xyc = (p_indep + 0.35 * elig * delta_syn_glob).clamp(0.05, 0.95);

        let pair_champ = self.get_counts(pack_combo_key(kind_champ, champ_id, ent_a, ent_b));
        let m_2 = 15.0;
        let w_xyc = pair_champ.wins as f64;
        let n_xyc = pair_champ.games as f64;

        let p_combo = ((w_xyc + m_2 * p_prior_xyc) / (n_xyc + m_2)).clamp(0.05, 0.95);

        let combo_wpa = p_combo - p_champ;
        let synergy_lift = p_combo - p_indep;

        let w_tilde = w_xyc + m_2 * p_prior_xyc;
        let n_tilde = n_xyc + m_2;
        let lcb_synergy_lift = wilson_lcb(w_tilde, n_tilde, 1.28155) - p_indep;

        (
            combo_wpa,
            synergy_lift,
            lcb_synergy_lift,
            pair_champ.games as usize,
        )
    }

    pub fn predict_build_winrate(
        &self,
        champ_id: u16,
        resources: &[EvaluatedResource],
        p_champ: f64,
    ) -> (f64, f64, Vec<ActivePairSynergyDto>, Vec<ActivePairSynergyDto>) {
        if resources.is_empty() {
            return (p_champ, 0.0, Vec::new(), Vec::new());
        }

        let l_0 = logit(p_champ);
        let mut delta_l_main = 0.0;
        for r in resources {
            let p_r = (p_champ + r.wpa).clamp(0.05, 0.95);
            delta_l_main += 0.75 * (logit(p_r) - l_0);
        }

        let mut active_combos = Vec::new();
        let mut anti_synergies = Vec::new();
        let mut synergy_lifts: Vec<f64> = Vec::new();

        for i in 0..resources.len() {
            for j in (i + 1)..resources.len() {
                let r1 = &resources[i];
                let r2 = &resources[j];
                let (combo_wpa, syn_lift, lcb_lift, sample_size) =
                    self.evaluate_pair_synergy(champ_id, r1, r2, p_champ);

                synergy_lifts.push(syn_lift);

                let label = match (r1.is_augment, r2.is_augment) {
                    (true, true) => {
                        let name1 = self.id_to_augment_string.get(&r1.id).map(String::as_str).unwrap_or("Aug");
                        let name2 = self.id_to_augment_string.get(&r2.id).map(String::as_str).unwrap_or("Aug");
                        format!("{name1} + {name2}")
                    }
                    (false, false) => format!("Item #{} + Item #{}", r1.id, r2.id),
                    (true, false) => {
                        let name = self.id_to_augment_string.get(&r1.id).map(String::as_str).unwrap_or("Aug");
                        format!("{name} + Item #{}", r2.id)
                    }
                    (false, true) => {
                        let name = self.id_to_augment_string.get(&r2.id).map(String::as_str).unwrap_or("Aug");
                        format!("{name} + Item #{}", r1.id)
                    }
                };

                let grade = compute_confidence_grade(syn_lift).to_string();

                let mut item_ids = Vec::new();
                let mut augment_ids = Vec::new();
                if r1.is_augment {
                    augment_ids.push(self.id_to_augment_string.get(&r1.id).cloned().unwrap_or_else(|| r1.id.to_string()));
                } else {
                    item_ids.push(r1.id);
                }
                if r2.is_augment {
                    augment_ids.push(self.id_to_augment_string.get(&r2.id).cloned().unwrap_or_else(|| r2.id.to_string()));
                } else {
                    item_ids.push(r2.id);
                }

                let combo_type = match (r1.is_augment, r2.is_augment) {
                    (true, true) => "aug_aug",
                    (false, false) => "item_item",
                    _ => "aug_item",
                }.to_string();

                let dto = ActivePairSynergyDto {
                    combo_type: combo_type.clone(),
                    item_ids: item_ids.clone(),
                    augment_ids: augment_ids.clone(),
                    label,
                    win_rate: p_champ + combo_wpa,
                    wpa: combo_wpa,
                    synergy_lift: syn_lift,
                    lcb_synergy_lift: lcb_lift,
                    sample_size,
                    confidence_grade: grade.clone(),
                    comboType: combo_type,
                    itemIds: item_ids,
                    augmentIds: augment_ids,
                    winRate: p_champ + combo_wpa,
                    synergyLift: syn_lift,
                    lcbSynergyLift: lcb_lift,
                    sampleSize: sample_size,
                    confidenceGrade: grade,
                };

                if syn_lift >= 0.0 {
                    active_combos.push(dto);
                } else if syn_lift < -0.015 {
                    anti_synergies.push(dto);
                }
            }
        }

        // Geometric decay on absolute synergy lifts
        synergy_lifts.sort_by(|a, b| b.abs().partial_cmp(&a.abs()).unwrap_or(std::cmp::Ordering::Equal));
        let mut delta_l_syn = 0.0;
        for (k, &syn_k) in synergy_lifts.iter().enumerate() {
            let decay = 0.70_f64.powi(k as i32);
            let p_syn = (p_champ + syn_k).clamp(0.05, 0.95);
            delta_l_syn += 0.85 * decay * (logit(p_syn) - l_0);
        }

        // Saturation with tanh (L_max = 1.50)
        let l_max = 1.50;
        let delta_l_sat = l_max * ((delta_l_main + delta_l_syn) / l_max).tanh();
        let p_build = sigmoid(l_0 + delta_l_sat).clamp(0.05, 0.95);
        let build_wpa = p_build - p_champ;

        active_combos.sort_by(|a, b| b.synergy_lift.partial_cmp(&a.synergy_lift).unwrap_or(std::cmp::Ordering::Equal));
        anti_synergies.sort_by(|a, b| a.synergy_lift.partial_cmp(&b.synergy_lift).unwrap_or(std::cmp::Ordering::Equal));

        (p_build, build_wpa, active_combos, anti_synergies)
    }

    pub fn compute_candidate_score(
        &self,
        champ_id: u16,
        candidate: &EvaluatedResource,
        locked_resources: &[EvaluatedResource],
        candidate_slot: u8,
        p_champ: f64,
        core_pool: &[EvaluatedResource],
    ) -> (f64, Option<u32>, Option<String>, Option<f64>) {
        let lambda_look = match candidate_slot {
            1 => 0.65,
            2 => 0.45,
            3 => 0.25,
            _ => 0.10,
        };

        let mut active_synergy_sum = 0.0;
        for locked in locked_resources {
            let (_, syn_lift, _, _) =
                self.evaluate_pair_synergy(champ_id, candidate, locked, p_champ);
            active_synergy_sum += syn_lift;
        }

        let mut best_lookahead_lift = 0.0;
        let mut best_partner_id = None;
        let mut best_partner_kind = None;

        for partner in core_pool {
            if partner.id == candidate.id && partner.is_augment == candidate.is_augment {
                continue;
            }
            if locked_resources.iter().any(|r| r.id == partner.id && r.is_augment == partner.is_augment) {
                continue;
            }
            let (_, syn_lift, _, _) =
                self.evaluate_pair_synergy(champ_id, candidate, partner, p_champ);
            if syn_lift > best_lookahead_lift {
                best_lookahead_lift = syn_lift;
                best_partner_id = Some(partner.id);
                best_partner_kind = Some(if partner.is_augment { "augment".to_string() } else { "item".to_string() });
            }
        }

        let n_rc_glob = candidate.games_glob as f64;
        let se_penalty = 0.50 * ((candidate.p_blend * (1.0 - candidate.p_blend)) / (n_rc_glob + 25.0)).sqrt();

        let score = candidate.wpa + active_synergy_sum + lambda_look * best_lookahead_lift - se_penalty;

        (
            score,
            best_partner_id,
            best_partner_kind,
            if best_lookahead_lift > 0.005 { Some(best_lookahead_lift) } else { None },
        )
    }
}

// --- Main Engine Recommender Entrypoint ---

pub fn compute_mayhem_recommendation(
    req: MayhemBuildRecommendationRequest,
    index: &MayhemSynergyIndex,
) -> MayhemBuildRecommendationResponse {
    let started = Instant::now();
    let champ_id = req.champion_id as u16;

    let p_champ = index.evaluate_champion_baseline(champ_id);

    // Parse locked augments
    let mut locked_aug_resources = Vec::new();
    for aug_str in &req.locked_augments {
        let num_id = parse_or_hash_augment_id(aug_str);
        let eval = index.evaluate_resource(champ_id, num_id, true, p_champ);
        locked_aug_resources.push(eval);
    }

    // Parse locked items
    let mut locked_item_resources = Vec::new();
    for &item_id in &req.locked_items {
        if item_id > 0 && !PORO_SNAX_IDS.contains(&item_id) {
            let eval = index.evaluate_resource(champ_id, item_id, false, p_champ);
            locked_item_resources.push(eval);
        }
    }

    let mut all_locked = Vec::with_capacity(locked_aug_resources.len() + locked_item_resources.len());
    all_locked.extend(locked_aug_resources.iter().copied());
    all_locked.extend(locked_item_resources.iter().copied());

    let (build_winrate, build_wpa, active_combos, anti_synergies) =
        index.predict_build_winrate(champ_id, &all_locked, p_champ);

    let augment_slot = req.augment_slot.unwrap_or_else(|| (locked_aug_resources.len() as u8 + 1).min(4));

    // Compile candidate core items pool for lookahead
    let candidate_item_ids: Vec<u32> = vec![
        3078, 3031, 3153, 3006, 3020, 3158, 3047, 3111, 3089, 3157, 3115, 3072, 3036, 3026, 3065,
        3083, 3742, 6653, 6655, 6673, 6672, 6632, 6691, 6692, 3142, 3139, 3100, 3116, 3165,
    ];
    let mut core_pool = Vec::new();
    for &id in &candidate_item_ids {
        if is_valid_build_item(id) || is_completed_boot(id) {
            core_pool.push(index.evaluate_resource(champ_id, id, false, p_champ));
        }
    }

    // 1. Evaluate 3-Card Augment draft if offered
    let mut evaluated_cards = Vec::new();
    let mut reroll_ev_dto = None;

    if let Some(ref cards) = req.offered_cards {
        if !cards.is_empty() {
            // Determine tier of offered cards (fallback "Gold")
            let first_meta = get_augment_meta(&cards[0]);
            let current_tier = first_meta.tier.clone();
            let is_upgrade = req.command_line_reroll.unwrap_or(false);
            let target_tier = if is_upgrade {
                match current_tier.as_str() {
                    "Silver" => "Gold".to_string(),
                    _ => "Prismatic".to_string(),
                }
            } else {
                current_tier.clone()
            };

            // Seen augments (including currently locked and offered cards)
            let mut seen_set: HashSet<String> = HashSet::new();
            if let Some(ref seen) = req.seen_augments {
                for s in seen {
                    seen_set.insert(s.to_lowercase());
                }
            }
            for a in &req.locked_augments {
                seen_set.insert(a.to_lowercase());
            }
            for c in cards {
                seen_set.insert(c.to_lowercase());
            }

            // Build unseen eligible pool in target_tier (deduplicating 3-entry metadata aliases)
            let mut eligible_unseen_scores = Vec::new();
            let mut processed_unseen_ids = HashSet::new();
            for (_key, meta) in AUGMENT_METADATA.iter() {
                if meta.tier != target_tier {
                    continue;
                }
                if !processed_unseen_ids.insert(meta.numeric_id) {
                    continue;
                }
                if seen_set.contains(&meta.id.to_lowercase())
                    || seen_set.contains(&meta.name.to_lowercase())
                    || seen_set.contains(&meta.numeric_id.to_string())
                {
                    continue;
                }
                let eval_cand = index.evaluate_resource(champ_id, meta.numeric_id, true, p_champ);
                if index.total_matches > 50 && eval_cand.games_glob == 0 {
                    continue;
                }
                let (sc, _, _, _) = index.compute_candidate_score(
                    champ_id,
                    &eval_cand,
                    &all_locked,
                    augment_slot,
                    p_champ,
                    &core_pool,
                );
                eligible_unseen_scores.push(sc);
            }

            let pool_empty = eligible_unseen_scores.is_empty();
            let ev_reroll = if !pool_empty {
                eligible_unseen_scores.iter().sum::<f64>() / eligible_unseen_scores.len() as f64
            } else {
                0.0
            };

            // Evaluate offered cards
            let mut card_scores: Vec<(AugmentMeta, EvaluatedResource, f64, Option<u32>, Option<String>, Option<f64>)> = Vec::new();
            for c in cards {
                let meta = get_augment_meta(c);
                let eval_cand = index.evaluate_resource(champ_id, meta.numeric_id, true, p_champ);
                let (sc, partner_id, partner_kind, lookahead_syn) = index.compute_candidate_score(
                    champ_id,
                    &eval_cand,
                    &all_locked,
                    augment_slot,
                    p_champ,
                    &core_pool,
                );
                card_scores.push((meta, eval_cand, sc, partner_id, partner_kind, lookahead_syn));
            }

            let max_score = card_scores.iter().map(|(_, _, sc, _, _, _)| *sc).fold(f64::NEG_INFINITY, f64::max);

            for (meta, eval_cand, sc, partner_id, partner_kind, lookahead_syn) in card_scores {
                let grade = compute_confidence_grade(sc).to_string();
                let is_best = sc >= max_score - 0.0001 && (pool_empty || sc >= ev_reroll);
                let is_reroll = !pool_empty && sc < ev_reroll;

                let verdict = if is_best {
                    "best_pick"
                } else if is_reroll {
                    "reroll"
                } else {
                    "neutral"
                }.to_string();

                let expected_gain = if is_reroll { Some(ev_reroll - sc) } else { None };

                let upgrade_prob = if !pool_empty {
                    let count_better = eligible_unseen_scores.iter().filter(|&&s| s > sc).count();
                    Some(count_better as f64 / eligible_unseen_scores.len() as f64)
                } else {
                    None
                };

                // Find active combos with locked items/augments
                let mut card_active_combos = Vec::new();
                for locked in &all_locked {
                    let (combo_wpa, syn_lift, lcb_lift, sample_size) =
                        index.evaluate_pair_synergy(champ_id, &eval_cand, locked, p_champ);
                    if syn_lift >= 0.01 {
                        let locked_label = if locked.is_augment {
                            index.id_to_augment_string.get(&locked.id).cloned().unwrap_or_else(|| locked.id.to_string())
                        } else {
                            format!("Item #{}", locked.id)
                        };
                        let combo_type = if locked.is_augment { "aug_aug" } else { "aug_item" }.to_string();
                        card_active_combos.push(ActivePairSynergyDto {
                            combo_type: combo_type.clone(),
                            item_ids: if locked.is_augment { vec![] } else { vec![locked.id] },
                            augment_ids: vec![meta.id.clone()],
                            label: format!("{} + {}", meta.name, locked_label),
                            win_rate: p_champ + combo_wpa,
                            wpa: combo_wpa,
                            synergy_lift: syn_lift,
                            lcb_synergy_lift: lcb_lift,
                            sample_size,
                            confidence_grade: compute_confidence_grade(syn_lift).to_string(),
                            comboType: combo_type,
                            itemIds: if locked.is_augment { vec![] } else { vec![locked.id] },
                            augmentIds: vec![meta.id.clone()],
                            winRate: p_champ + combo_wpa,
                            synergyLift: syn_lift,
                            lcbSynergyLift: lcb_lift,
                            sampleSize: sample_size,
                            confidenceGrade: compute_confidence_grade(syn_lift).to_string(),
                        });
                    }
                }

                evaluated_cards.push(AugmentCardEvalDto {
                    augment_id: meta.id.clone(),
                    name: meta.name.clone(),
                    icon_url: meta.icon_url.clone(),
                    tier: meta.tier.clone(),
                    tooltip: meta.tooltip.clone(),
                    score: sc,
                    wpa: eval_cand.wpa,
                    kit_lift: eval_cand.delta_champ,
                    lcb_wpa: eval_cand.lcb_wpa,
                    sample_size: eval_cand.games_glob as usize,
                    verdict: verdict.clone(),
                    confidence_grade: grade.clone(),
                    expected_reroll_gain: expected_gain,
                    upgrade_probability: upgrade_prob,
                    active_combos: card_active_combos.clone(),
                    lookahead_partner_id: partner_id,
                    lookahead_partner_kind: partner_kind.clone(),
                    lookahead_synergy_lift: lookahead_syn,
                    augmentId: meta.id,
                    iconUrl: meta.icon_url,
                    kitLift: eval_cand.delta_champ,
                    lcbWpa: eval_cand.lcb_wpa,
                    sampleSize: eval_cand.games_glob as usize,
                    confidenceGrade: grade,
                    expectedRerollGain: expected_gain,
                    upgradeProbability: upgrade_prob,
                    activeCombos: card_active_combos,
                    lookaheadPartnerId: partner_id,
                    lookaheadPartnerKind: partner_kind,
                    lookaheadSynergyLift: lookahead_syn,
                });
            }

            let best_pick_id = evaluated_cards.iter().find(|c| c.verdict == "best_pick").map(|c| c.augment_id.clone());
            let should_reroll = !pool_empty && max_score < ev_reroll;

            reroll_ev_dto = Some(RerollEvDto {
                ev_reroll,
                pool_size: eligible_unseen_scores.len(),
                tier: current_tier.clone(),
                target_tier: target_tier.clone(),
                best_pick_id: best_pick_id.clone(),
                should_reroll,
                evReroll: ev_reroll,
                poolSize: eligible_unseen_scores.len(),
                targetTier: target_tier,
                bestPickId: best_pick_id,
                shouldReroll: should_reroll,
            });
        }
    }

    // 2. Recommend Next Augments by Tier
    let mut recommended_augments_by_tier: HashMap<String, Vec<AugmentRecommendationDto>> = HashMap::new();
    let tiers = ["Silver", "Gold", "Prismatic"];
    for &t in &tiers {
        let mut tier_candidates = Vec::new();
        let mut processed_names = HashSet::new();

        for (_, meta) in AUGMENT_METADATA.iter() {
            if meta.tier == t && !processed_names.contains(&meta.name) {
                processed_names.insert(meta.name.clone());
                if req.locked_augments.iter().any(|a| a.eq_ignore_ascii_case(&meta.name) || a == &meta.id) {
                    continue;
                }
                let eval_cand = index.evaluate_resource(champ_id, meta.numeric_id, true, p_champ);
                let (sc, partner_id, partner_kind, lookahead_syn) = index.compute_candidate_score(
                    champ_id,
                    &eval_cand,
                    &all_locked,
                    augment_slot,
                    p_champ,
                    &core_pool,
                );
                tier_candidates.push((meta, eval_cand, sc, partner_id, partner_kind, lookahead_syn));
            }
        }

        tier_candidates.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));
        tier_candidates.truncate(6);

        let list: Vec<AugmentRecommendationDto> = tier_candidates
            .into_iter()
            .map(|(meta, eval, sc, partner_id, partner_kind, lookahead_syn)| {
                let grade = compute_confidence_grade(sc).to_string();
                AugmentRecommendationDto {
                    augment_id: meta.id.clone(),
                    name: meta.name.clone(),
                    icon_url: meta.icon_url.clone(),
                    tier: meta.tier.clone(),
                    tooltip: meta.tooltip.clone(),
                    score: sc,
                    wpa: eval.wpa,
                    kit_lift: eval.delta_champ,
                    lcb_wpa: eval.lcb_wpa,
                    sample_size: eval.games_glob as usize,
                    confidence_grade: grade.clone(),
                    lookahead_partner_id: partner_id,
                    lookahead_partner_kind: partner_kind.clone(),
                    lookahead_synergy_lift: lookahead_syn,
                    augmentId: meta.id.clone(),
                    iconUrl: meta.icon_url.clone(),
                    kitLift: eval.delta_champ,
                    lcbWpa: eval.lcb_wpa,
                    sampleSize: eval.games_glob as usize,
                    confidenceGrade: grade,
                    lookaheadPartnerId: partner_id,
                    lookaheadPartnerKind: partner_kind,
                    lookaheadSynergyLift: lookahead_syn,
                }
            })
            .collect();

        recommended_augments_by_tier.insert(t.to_string(), list);
    }

    // 3. Evaluate Next Items, Boots & Completed Build
    let all_candidate_items: Vec<u32> = vec![
        3078, 3031, 3153, 3089, 3157, 3115, 3072, 3036, 3026, 3065, 3083, 3742,
        6653, 6655, 6673, 6672, 6632, 6691, 6692, 3142, 3139, 3100, 3116, 3165,
        3074, 3071, 3143, 3075, 3110, 3107, 3508, 3004, 3119, 3152, 4637, 6694,
    ];
    let boot_candidate_ids: Vec<u32> = vec![3006, 3020, 3158, 3047, 3111, 3009, 3117];

    let has_boots = req.locked_items.iter().any(|&id| is_completed_boot(id));

    let mut recommended_boots = None;
    if !has_boots {
        let mut boot_evals = Vec::new();
        for &id in &boot_candidate_ids {
            let eval = index.evaluate_resource(champ_id, id, false, p_champ);
            let mut syn_sum = 0.0;
            for locked in &all_locked {
                let (_, syn_lift, _, _) = index.evaluate_pair_synergy(champ_id, &eval, locked, p_champ);
                syn_sum += syn_lift;
            }
            let total_score = eval.wpa + syn_sum;
            boot_evals.push((id, eval, total_score, syn_sum));
        }
        boot_evals.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));
        if let Some(&(id, ref eval, total_sc, syn_sum)) = boot_evals.first() {
            let grade = compute_confidence_grade(total_sc).to_string();
            recommended_boots = Some(ItemRecommendationDto {
                item_id: id,
                win_rate: p_champ + eval.wpa,
                lift: eval.wpa,
                sample_size: eval.games_glob as usize,
                is_boot: true,
                reason: format!("High synergy boot (+{:.1}% lift)", syn_sum * 100.0),
                synergy_lift: syn_sum,
                lcb_lift: eval.lcb_wpa,
                confidence_grade: grade.clone(),
                itemId: id,
                winRate: p_champ + eval.wpa,
                sampleSize: eval.games_glob as usize,
                isBoot: true,
                synergyLift: syn_sum,
                lcbLift: eval.lcb_wpa,
                confidenceGrade: grade,
            });
        }
    }

    let mut scored_items = Vec::new();
    for &id in &all_candidate_items {
        if req.locked_items.contains(&id) || is_completed_boot(id) || !is_legendary_item(id) {
            continue;
        }
        let eval = index.evaluate_resource(champ_id, id, false, p_champ);
        let mut syn_sum = 0.0;
        for locked in &all_locked {
            let (_, syn_lift, _, _) = index.evaluate_pair_synergy(champ_id, &eval, locked, p_champ);
            syn_sum += syn_lift;
        }
        let cost = get_gold_cost(id).max(2000) as f64;
        let cost_scaling = 3000.0 / cost;
        let total_score = (eval.wpa + syn_sum) * cost_scaling;
        scored_items.push((id, eval, total_score, syn_sum));
    }

    scored_items.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));

    let mut next_items = Vec::new();
    let mut alternative_items = Vec::new();

    for (idx, &(id, ref eval, total_sc, syn_sum)) in scored_items.iter().enumerate() {
        let grade = compute_confidence_grade(total_sc).to_string();
        let dto = ItemRecommendationDto {
            item_id: id,
            win_rate: p_champ + eval.wpa,
            lift: eval.wpa,
            sample_size: eval.games_glob as usize,
            is_boot: false,
            reason: format!("Empirical Bayes lift: +{:.1}% synergy", syn_sum * 100.0),
            synergy_lift: syn_sum,
            lcb_lift: eval.lcb_wpa,
            confidence_grade: grade.clone(),
            itemId: id,
            winRate: p_champ + eval.wpa,
            sampleSize: eval.games_glob as usize,
            isBoot: false,
            synergyLift: syn_sum,
            lcbLift: eval.lcb_wpa,
            confidenceGrade: grade,
        };
        if idx < 3 {
            next_items.push(dto);
        } else if idx < 8 {
            alternative_items.push(dto);
        }
    }

    // Build completed 6-item target build
    let mut completed_build = Vec::new();
    for &id in &req.locked_items {
        completed_build.push(ItemRecommendationDto {
            item_id: id,
            win_rate: p_champ,
            lift: 0.0,
            sample_size: 100,
            is_boot: is_completed_boot(id),
            reason: "Currently slotted item".to_string(),
            synergy_lift: 0.0,
            lcb_lift: 0.0,
            confidence_grade: "B".to_string(),
            itemId: id,
            winRate: p_champ,
            sampleSize: 100,
            isBoot: is_completed_boot(id),
            synergyLift: 0.0,
            lcbLift: 0.0,
            confidenceGrade: "B".to_string(),
        });
    }

    if let Some(ref boot) = recommended_boots {
        if completed_build.len() < 6 && !completed_build.iter().any(|i| i.is_boot) {
            completed_build.push(boot.clone());
        }
    }

    for item in &next_items {
        if completed_build.len() >= 6 {
            break;
        }
        if !completed_build.iter().any(|i| i.item_id == item.item_id) {
            completed_build.push(item.clone());
        }
    }
    for item in &alternative_items {
        if completed_build.len() >= 6 {
            break;
        }
        if !completed_build.iter().any(|i| i.item_id == item.item_id) {
            completed_build.push(item.clone());
        }
    }

    let elapsed = started.elapsed().as_secs_f64() * 1000.0;
    let stats = EnginePerformanceStatsDto {
        load_time_ms: 0.5,
        compute_time_ms: elapsed,
        total_time_ms: elapsed,
        match_count: index.total_matches as usize,
        cache_hit: true,
        loadTimeMs: 0.5,
        computeTimeMs: elapsed,
        totalTimeMs: elapsed,
        matchCount: index.total_matches as usize,
        cacheHit: true,
    };

    MayhemBuildRecommendationResponse {
        champion_id: req.champion_id,
        champion_baseline_winrate: p_champ,
        build_winrate,
        build_wpa,
        active_combos: active_combos.clone(),
        anti_synergies: anti_synergies.clone(),
        evaluated_cards: evaluated_cards.clone(),
        reroll_ev: reroll_ev_dto.clone(),
        next_items: next_items.clone(),
        recommended_boots: recommended_boots.clone(),
        completed_build: completed_build.clone(),
        alternative_items: alternative_items.clone(),
        recommended_augments_by_tier: recommended_augments_by_tier.clone(),
        stats: stats.clone(),

        championId: req.champion_id,
        championBaselineWinrate: p_champ,
        buildWinrate: build_winrate,
        buildWpa: build_wpa,
        activeCombos: active_combos,
        antiSynergies: anti_synergies,
        evaluatedCards: evaluated_cards,
        rerollEv: reroll_ev_dto,
        nextItems: next_items,
        recommendedBoots: recommended_boots,
        completedBuild: completed_build,
        alternativeItems: alternative_items,
        recommendedAugmentsByTier: recommended_augments_by_tier,
    }
}

// --- Query Mayhem Combos Implementation ---

pub fn query_mayhem_combos(
    index: &MayhemSynergyIndex,
    champion_id: Option<u16>,
    category: Option<&str>,
    sort_by: Option<&str>,
    descending: bool,
    limit: usize,
    offset: usize,
) -> Vec<MayhemComboDto> {
    let mut results = Vec::new();
    let cat = category.unwrap_or("All");
    let target_champ = champion_id.unwrap_or(0);

    for (&key, &count) in &index.counts {
        if count.games < 2 {
            continue;
        }

        let (kind, c_id, ent_a, ent_b) = unpack_combo_key(key);

        if target_champ > 0 && c_id > 0 && c_id != target_champ {
            continue;
        }

        let is_three_way = c_id > 0
            && (kind == KIND_ITEM_AUG_CHAMP
                || kind == KIND_AUG_AUG_CHAMP
                || kind == KIND_ITEM_ITEM_CHAMP);

        let base_category = match kind {
            KIND_ITEM_AUG_GLOBAL | KIND_ITEM_AUG_CHAMP => "ItemAug",
            KIND_AUG_AUG_GLOBAL | KIND_AUG_AUG_CHAMP => "AugAug",
            KIND_ITEM_ITEM_GLOBAL | KIND_ITEM_ITEM_CHAMP => "ItemItem",
            KIND_AUG_CHAMP => "AugChamp",
            KIND_ITEM_CHAMP => "ItemChamp",
            _ => continue,
        };

        let matches_category = if cat == "All" {
            true
        } else if cat == "ThreeWayChamp" {
            is_three_way
        } else {
            cat == base_category
        };

        if !matches_category {
            continue;
        }

        let combo_category = if is_three_way && cat == "ThreeWayChamp" {
            "ThreeWayChamp"
        } else {
            base_category
        };

        // Build components list
        let mut components = Vec::new();
        if c_id > 0 {
            components.push(ComboComponentDto {
                kind: "champion".to_string(),
                id: serde_json::Value::from(c_id),
                name: format!("Champion #{c_id}"),
                icon_url: format!(
                    "https://raw.communitydragon.org/latest/plugins/rcp-be-lol-game-data/global/default/v1/champion-icons/{}.png",
                    c_id
                ),
                rarity: None,
                item_class: None,
                description: None,
                price_total: None,
                categories: None,
            });
        }

        match kind {
            KIND_AUG_CHAMP | KIND_AUG_SINGLE => {
                let aug_str = index.id_to_augment_string.get(&ent_a).map(String::as_str).unwrap_or("");
                let meta = get_augment_meta(aug_str);
                components.push(ComboComponentDto {
                    kind: "augment".to_string(),
                    id: serde_json::Value::String(meta.id.clone()),
                    name: meta.name.clone(),
                    icon_url: meta.icon_url.clone(),
                    rarity: Some(meta.tier.clone()),
                    item_class: None,
                    description: Some(meta.tooltip.clone()),
                    price_total: None,
                    categories: None,
                });
            }
            KIND_ITEM_CHAMP | KIND_ITEM_SINGLE => {
                components.push(ComboComponentDto {
                    kind: "item".to_string(),
                    id: serde_json::Value::from(ent_a),
                    name: format!("Item #{ent_a}"),
                    icon_url: format!(
                        "https://raw.communitydragon.org/latest/plugins/rcp-be-lol-game-data/global/default/assets/items/icons2d/{}.png",
                        ent_a
                    ).to_lowercase(),
                    rarity: None,
                    item_class: Some("Legendary".to_string()),
                    description: None,
                    price_total: Some(get_gold_cost(ent_a)),
                    categories: Some(get_categories(ent_a)),
                });
            }
            KIND_AUG_AUG_GLOBAL | KIND_AUG_AUG_CHAMP => {
                let aug1_str = index.id_to_augment_string.get(&ent_a).map(String::as_str).unwrap_or("");
                let meta1 = get_augment_meta(aug1_str);
                let aug2_str = index.id_to_augment_string.get(&ent_b).map(String::as_str).unwrap_or("");
                let meta2 = get_augment_meta(aug2_str);
                components.push(ComboComponentDto {
                    kind: "augment".to_string(),
                    id: serde_json::Value::String(meta1.id.clone()),
                    name: meta1.name.clone(),
                    icon_url: meta1.icon_url.clone(),
                    rarity: Some(meta1.tier.clone()),
                    item_class: None,
                    description: Some(meta1.tooltip.clone()),
                    price_total: None,
                    categories: None,
                });
                components.push(ComboComponentDto {
                    kind: "augment".to_string(),
                    id: serde_json::Value::String(meta2.id.clone()),
                    name: meta2.name.clone(),
                    icon_url: meta2.icon_url.clone(),
                    rarity: Some(meta2.tier.clone()),
                    item_class: None,
                    description: Some(meta2.tooltip.clone()),
                    price_total: None,
                    categories: None,
                });
            }
            KIND_ITEM_ITEM_GLOBAL | KIND_ITEM_ITEM_CHAMP => {
                components.push(ComboComponentDto {
                    kind: "item".to_string(),
                    id: serde_json::Value::from(ent_a),
                    name: format!("Item #{ent_a}"),
                    icon_url: format!(
                        "https://raw.communitydragon.org/latest/plugins/rcp-be-lol-game-data/global/default/assets/items/icons2d/{}.png",
                        ent_a
                    ).to_lowercase(),
                    rarity: None,
                    item_class: Some("Legendary".to_string()),
                    description: None,
                    price_total: Some(get_gold_cost(ent_a)),
                    categories: Some(get_categories(ent_a)),
                });
                components.push(ComboComponentDto {
                    kind: "item".to_string(),
                    id: serde_json::Value::from(ent_b),
                    name: format!("Item #{ent_b}"),
                    icon_url: format!(
                        "https://raw.communitydragon.org/latest/plugins/rcp-be-lol-game-data/global/default/assets/items/icons2d/{}.png",
                        ent_b
                    ).to_lowercase(),
                    rarity: None,
                    item_class: Some("Legendary".to_string()),
                    description: None,
                    price_total: Some(get_gold_cost(ent_b)),
                    categories: Some(get_categories(ent_b)),
                });
            }
            KIND_ITEM_AUG_GLOBAL | KIND_ITEM_AUG_CHAMP => {
                let aug_str = index.id_to_augment_string.get(&ent_a).map(String::as_str).unwrap_or("");
                let meta = get_augment_meta(aug_str);
                components.push(ComboComponentDto {
                    kind: "augment".to_string(),
                    id: serde_json::Value::String(meta.id.clone()),
                    name: meta.name.clone(),
                    icon_url: meta.icon_url.clone(),
                    rarity: Some(meta.tier.clone()),
                    item_class: None,
                    description: Some(meta.tooltip.clone()),
                    price_total: None,
                    categories: None,
                });
                components.push(ComboComponentDto {
                    kind: "item".to_string(),
                    id: serde_json::Value::from(ent_b),
                    name: format!("Item #{ent_b}"),
                    icon_url: format!(
                        "https://raw.communitydragon.org/latest/plugins/rcp-be-lol-game-data/global/default/assets/items/icons2d/{}.png",
                        ent_b
                    ).to_lowercase(),
                    rarity: None,
                    item_class: Some("Legendary".to_string()),
                    description: None,
                    price_total: Some(get_gold_cost(ent_b)),
                    categories: Some(get_categories(ent_b)),
                });
            }
            _ => continue,
        }

        let p_champ = if c_id > 0 {
            index.evaluate_champion_baseline(c_id)
        } else {
            0.50
        };

        let raw_win_rate = if count.games > 0 {
            count.wins as f64 / count.games as f64
        } else {
            0.50
        };

        let (combo_wpa, synergy_lift, lcb_synergy_lift) = match kind {
            KIND_AUG_CHAMP => {
                let r = index.evaluate_resource(c_id, ent_a, true, p_champ);
                (r.wpa, r.delta_champ, r.lcb_wpa)
            }
            KIND_ITEM_CHAMP => {
                let r = index.evaluate_resource(c_id, ent_a, false, p_champ);
                (r.wpa, r.delta_champ, r.lcb_wpa)
            }
            KIND_AUG_AUG_GLOBAL | KIND_AUG_AUG_CHAMP => {
                let r1 = index.evaluate_resource(c_id, ent_a, true, p_champ);
                let r2 = index.evaluate_resource(c_id, ent_b, true, p_champ);
                let (cw, syn, lcb, _) = index.evaluate_pair_synergy(c_id, &r1, &r2, p_champ);
                (cw, syn, lcb)
            }
            KIND_ITEM_ITEM_GLOBAL | KIND_ITEM_ITEM_CHAMP => {
                let r1 = index.evaluate_resource(c_id, ent_a, false, p_champ);
                let r2 = index.evaluate_resource(c_id, ent_b, false, p_champ);
                let (cw, syn, lcb, _) = index.evaluate_pair_synergy(c_id, &r1, &r2, p_champ);
                (cw, syn, lcb)
            }
            KIND_ITEM_AUG_GLOBAL | KIND_ITEM_AUG_CHAMP => {
                let r1 = index.evaluate_resource(c_id, ent_a, true, p_champ);
                let r2 = index.evaluate_resource(c_id, ent_b, false, p_champ);
                let (cw, syn, lcb, _) = index.evaluate_pair_synergy(c_id, &r1, &r2, p_champ);
                (cw, syn, lcb)
            }
            _ => continue,
        };

        let grade = compute_confidence_grade(synergy_lift).to_string();

        let id = format!("{combo_category}:{c_id}:{ent_a}:{ent_b}");
        let name = components.iter().map(|c| c.name.clone()).collect::<Vec<_>>().join(" + ");

        results.push(MayhemComboDto {
            id,
            name,
            category: combo_category.to_string(),
            champion_id: c_id,
            components,
            games: count.games,
            wins: count.wins,
            win_rate: raw_win_rate,
            combo_wpa,
            wpa: combo_wpa,
            synergy_lift,
            lift: synergy_lift,
            synergy_score: synergy_lift,
            lcb_synergy_lift,
            confidence_grade: grade.clone(),

            championId: c_id,
            winRate: raw_win_rate,
            comboWpa: combo_wpa,
            synergyLift: synergy_lift,
            synergyScore: synergy_lift,
            lcbSynergyLift: lcb_synergy_lift,
            confidenceGrade: grade,
        });
    }

    // Sort
    let sort = sort_by.unwrap_or("lcbWpa");
    results.sort_by(|a, b| {
        let cmp = match sort {
            "winRate" | "win_rate" => a.win_rate.partial_cmp(&b.win_rate),
            "games" => a.games.partial_cmp(&b.games),
            "synergyLift" | "synergy_lift" => a.synergy_lift.partial_cmp(&b.synergy_lift),
            "comboWpa" | "combo_wpa" | "wpa" => a.combo_wpa.partial_cmp(&b.combo_wpa),
            _ => a.lcb_synergy_lift.partial_cmp(&b.lcb_synergy_lift),
        }
        .unwrap_or(std::cmp::Ordering::Equal);

        if descending {
            cmp.reverse()
        } else {
            cmp
        }
    });

    results.into_iter().skip(offset).take(limit).collect()
}

// --- Overlay card decisions (v3 math) ---

use crate::mayhem_math::{
    estimate_tau2, evaluate_reroll, logit as math_logit, pair_effect, sample_effect,
    sigmoid as math_sigmoid, PairEffect, RerollEval, SingleEffect, TAU2_FLOOR, Z80,
};

/// Reroll is only advised when the expected log-odds gain clears this threshold
/// (0.02 nats ~ 0.5% win rate).
pub const REROLL_SIGNIFICANCE: f64 = 0.02;

#[derive(Debug, Clone)]
pub struct CardDecision {
    pub augment_id: u32,
    pub effect: f64,
    pub win_rate: f64,
    pub win_rate_low: f64,
    pub win_rate_high: f64,
    pub wpa: f64,
    pub games: u32,
    pub verdict: &'static str,
    pub rank: u8,
    pub reroll_gain: f64,
    pub reroll_probability: f64,
}

/// Returns one verdict per candidate score and the reroll evaluation it used.
pub fn apply_verdicts(
    candidate_scores: &[f64],
    pool_scores: &[f64],
    draw_count: usize,
    margin: f64,
) -> (Vec<&'static str>, RerollEval) {
    let best = candidate_scores
        .iter()
        .cloned()
        .fold(f64::NEG_INFINITY, f64::max);
    let eval = evaluate_reroll(pool_scores, best, draw_count);
    let reroll_best = eval.drawable && eval.gain > margin;

    let verdicts = candidate_scores
        .iter()
        .map(|score| {
            if (score - best).abs() < 1e-9 {
                if reroll_best {
                    "reroll"
                } else {
                    "pick"
                }
            } else {
                "skip"
            }
        })
        .collect();

    (verdicts, eval)
}

fn resource_counts(
    index: &MayhemSynergyIndex,
    champ_id: u16,
    id: u32,
    is_augment: bool,
) -> (u32, u32) {
    let kind = if is_augment {
        KIND_AUG_CHAMP
    } else {
        KIND_ITEM_CHAMP
    };
    let c = index.get_counts(pack_combo_key(kind, champ_id, id, 0));
    (c.wins, c.games)
}

fn pair_counts(
    index: &MayhemSynergyIndex,
    champ_id: u16,
    a: (u32, bool),
    b: (u32, bool),
) -> (u32, u32) {
    let (kind, entity_a, entity_b) = match (a.1, b.1) {
        (true, true) => (KIND_AUG_AUG_CHAMP, a.0.min(b.0), a.0.max(b.0)),
        (false, false) => (KIND_ITEM_ITEM_CHAMP, a.0.min(b.0), a.0.max(b.0)),
        (true, false) => (KIND_ITEM_AUG_CHAMP, a.0, b.0),
        (false, true) => (KIND_ITEM_AUG_CHAMP, b.0, a.0),
    };
    let c = index.get_counts(pack_combo_key(kind, champ_id, entity_a, entity_b));
    (c.wins, c.games)
}

fn population_tau2(
    index: &MayhemSynergyIndex,
    champ_id: u16,
    p_champ: f64,
    is_augment: bool,
) -> f64 {
    let kind = if is_augment {
        KIND_AUG_CHAMP
    } else {
        KIND_ITEM_CHAMP
    };
    let mut samples = Vec::new();
    for (key, counts) in index.counts.iter() {
        let (k, champ, _, entity_b) = unpack_combo_key(*key);
        if k == kind && champ == champ_id && entity_b == 0 && counts.games > 0 {
            samples.push((counts.wins, counts.games));
        }
    }
    estimate_tau2(p_champ, &samples)
}

struct LockedResource {
    id: u32,
    is_augment: bool,
    effect: SingleEffect,
}

fn score_candidate(
    index: &MayhemSynergyIndex,
    champ_id: u16,
    p_champ: f64,
    tau2_aug: f64,
    tau2_item: f64,
    locked: &[LockedResource],
    candidate: (u32, bool),
) -> (SingleEffect, f64) {
    let tau2 = if candidate.1 { tau2_aug } else { tau2_item };
    let (wins, games) = resource_counts(index, champ_id, candidate.0, candidate.1);
    let effect = sample_effect(p_champ, wins, games, tau2);
    let base = math_logit(p_champ);

    let mut pair_sum = 0.0;
    for locked_res in locked {
        let (pw, pn) = pair_counts(
            index,
            champ_id,
            candidate,
            (locked_res.id, locked_res.is_augment),
        );
        if pn < 5 {
            continue;
        }
        let pe: PairEffect = pair_effect(
            base + effect.effect + locked_res.effect.effect,
            pw,
            pn,
            effect.posterior_var,
            locked_res.effect.posterior_var,
            TAU2_FLOOR,
        );
        pair_sum += pe.effect;
    }

    (effect, pair_sum)
}

/// Score every offered card, rank them, and decide pick vs reroll.
pub fn decide_offered_cards(
    index: &MayhemSynergyIndex,
    champ_id: u16,
    offered: &[u32],
    locked_augments: &[u32],
    locked_items: &[u32],
    tier: &str,
    seen: &[u32],
) -> Vec<CardDecision> {
    if offered.is_empty() {
        return Vec::new();
    }

    let p_champ = index.evaluate_champion_baseline(champ_id);
    let l0 = math_logit(p_champ);
    let tau2_aug = population_tau2(index, champ_id, p_champ, true);
    let tau2_item = population_tau2(index, champ_id, p_champ, false);

    let mut locked: Vec<LockedResource> = Vec::new();
    for &id in locked_augments {
        let (wins, games) = resource_counts(index, champ_id, id, true);
        locked.push(LockedResource {
            id,
            is_augment: true,
            effect: sample_effect(p_champ, wins, games, tau2_aug),
        });
    }
    for &id in locked_items {
        if id == 0 || PORO_SNAX_IDS.contains(&id) {
            continue;
        }
        let (wins, games) = resource_counts(index, champ_id, id, false);
        locked.push(LockedResource {
            id,
            is_augment: false,
            effect: sample_effect(p_champ, wins, games, tau2_item),
        });
    }

    let mut candidate_effects = Vec::with_capacity(offered.len());
    let mut candidate_scores = Vec::with_capacity(offered.len());
    for &id in offered {
        let (effect, pair_sum) = score_candidate(
            index,
            champ_id,
            p_champ,
            tau2_aug,
            tau2_item,
            &locked,
            (id, true),
        );
        candidate_effects.push(effect);
        candidate_scores.push(effect.effect + pair_sum);
    }

    let mut excluded: HashSet<u32> = seen.iter().copied().collect();
    excluded.extend(offered.iter().copied());

    let mut pool_scores = Vec::new();
    let mut dedupe: HashSet<u32> = HashSet::new();
    for meta in AUGMENT_METADATA.values() {
        if meta.tier != tier
            || !dedupe.insert(meta.numeric_id)
            || excluded.contains(&meta.numeric_id)
        {
            continue;
        }
        let (effect, pair_sum) = score_candidate(
            index,
            champ_id,
            p_champ,
            tau2_aug,
            tau2_item,
            &locked,
            (meta.numeric_id, true),
        );
        pool_scores.push(effect.effect + pair_sum);
    }

    let (verdicts, eval) = apply_verdicts(&candidate_scores, &pool_scores, 3, REROLL_SIGNIFICANCE);

    let mut decisions = Vec::with_capacity(offered.len());
    for i in 0..offered.len() {
        let effect = candidate_effects[i];
        let score = candidate_scores[i];
        let log_odds = l0 + effect.effect;
        let p = math_sigmoid(log_odds).clamp(0.01, 0.99);
        let se = effect.posterior_var.sqrt();
        let rank = candidate_scores.iter().filter(|s| **s > score).count() + 1;
        decisions.push(CardDecision {
            augment_id: offered[i],
            effect: effect.effect,
            win_rate: p,
            win_rate_low: math_sigmoid(log_odds - Z80 * se),
            win_rate_high: math_sigmoid(log_odds + Z80 * se),
            wpa: p - p_champ,
            games: effect.games,
            verdict: verdicts[i],
            rank: rank as u8,
            reroll_gain: eval.gain,
            reroll_probability: eval.improve_probability,
        });
    }

    decisions
}

#[cfg(test)]
mod tests {
    use super::*;

    fn synthetic_index() -> MayhemSynergyIndex {
        let mut index = MayhemSynergyIndex::default();
        index.total_matches = 1000;
        index.counts.insert(
            pack_combo_key(KIND_CHAMP_BASE, 1, 0, 0),
            WinGameCount {
                wins: 500,
                games: 1000,
            },
        );
        index.counts.insert(
            pack_combo_key(KIND_AUG_CHAMP, 1, 40, 0),
            WinGameCount {
                wins: 60,
                games: 100,
            },
        );
        index.counts.insert(
            pack_combo_key(KIND_AUG_CHAMP, 1, 41, 0),
            WinGameCount {
                wins: 45,
                games: 100,
            },
        );
        index.id_to_augment_string.insert(40, "Test A".to_string());
        index.id_to_augment_string.insert(41, "Test B".to_string());
        index
    }

    #[test]
    fn picks_when_the_pool_cannot_beat_the_best_card() {
        let (verdicts, eval) = apply_verdicts(&[0.30, 0.10], &[0.0, 0.05], 3, REROLL_SIGNIFICANCE);
        assert_eq!(verdicts, vec!["pick", "skip"]);
        assert!(!eval.drawable);
    }

    #[test]
    fn rerolls_when_the_pool_is_clearly_better() {
        let pool = [0.40, 0.35, 0.30, 0.25];
        let (verdicts, eval) = apply_verdicts(&[0.10, 0.05], &pool, 3, REROLL_SIGNIFICANCE);
        assert_eq!(verdicts[0], "reroll");
        assert_eq!(verdicts[1], "skip");
        assert!(eval.gain > REROLL_SIGNIFICANCE);
        assert_eq!(eval.improve_probability, 1.0);
    }

    #[test]
    fn verdicts_track_the_highest_score_only() {
        let (verdicts, _) = apply_verdicts(&[0.05, 0.02, 0.01], &[], 3, REROLL_SIGNIFICANCE);
        assert_eq!(verdicts, vec!["pick", "skip", "skip"]);
    }

    #[test]
    fn picks_the_higher_winrate_augment() {
        let index = synthetic_index();
        let decisions = decide_offered_cards(&index, 1, &[40, 41], &[], &[], "Gold", &[]);
        assert_eq!(decisions.len(), 2);
        assert!(decisions[0].wpa > decisions[1].wpa);
        assert_eq!(decisions[0].rank, 1);
        assert_eq!(decisions[1].rank, 2);
        assert_eq!(decisions[0].verdict, "pick");
        assert_eq!(decisions[1].verdict, "skip");
    }

    #[test]
    fn card_winrate_is_anchored_on_the_champion_baseline() {
        let index = synthetic_index();
        let decisions = decide_offered_cards(&index, 1, &[40], &[], &[], "Gold", &[]);
        let p_champ = index.evaluate_champion_baseline(1);
        assert!((decisions[0].win_rate - p_champ - decisions[0].wpa).abs() < 1e-9);
        assert!(decisions[0].win_rate_low < decisions[0].win_rate);
        assert!(decisions[0].win_rate_high > decisions[0].win_rate);
    }
}
