use std::collections::HashMap;
use std::time::Instant;

use crate::build_engine::data_loader::RawParticipantMatch;
use crate::build_engine::models::{
    ActiveComboSynergy, BuildEngineRequest, BuildRecommendationResult, EnginePerformanceStats,
    ItemRecommendation,
};

pub fn is_boot_item_id(id: u32) -> bool {
    crate::build_engine::item_catalog::is_completed_boot(id)
}

pub fn is_starter_item_id(id: u32) -> bool {
    matches!(
        id,
        1054 | 1055 | 1056 | 1082 | 2051 | 3177 | 3184 | 3112 | 3070
    )
}

pub fn is_excluded_item_id(id: u32) -> bool {
    !crate::build_engine::item_catalog::is_valid_build_item(id)
}

#[derive(Debug, Clone)]
pub struct ChampionTransactionIndex {
    pub champion_id: u32,
    pub total_matches: usize,
    pub champion_wins: usize,
    pub champion_win_rate: f64,
    pub win_mask: Vec<u64>,
    pub all_mask: Vec<u64>,
    pub item_bitsets: HashMap<u32, Vec<u64>>,
    pub augment_bitsets: HashMap<String, Vec<u64>>,
    pub item_counts: HashMap<u32, (usize, usize)>, // (games, wins)
    pub augment_counts: HashMap<String, (usize, usize)>, // (games, wins)
}

impl ChampionTransactionIndex {
    pub fn from_matches(champion_id: u32, matches: &[RawParticipantMatch]) -> Self {
        let total = matches.len();
        let num_words = (total + 63) / 64;

        let mut win_mask = vec![0u64; num_words];
        let mut all_mask = vec![0u64; num_words];
        let mut item_bitsets: HashMap<u32, Vec<u64>> = HashMap::new();
        let mut augment_bitsets: HashMap<String, Vec<u64>> = HashMap::new();
        let mut item_counts: HashMap<u32, (usize, usize)> = HashMap::new();
        let mut augment_counts: HashMap<String, (usize, usize)> = HashMap::new();
        let mut wins = 0;

        for (idx, m) in matches.iter().enumerate() {
            let word_idx = idx / 64;
            let bit_idx = idx % 64;
            let mask = 1u64 << bit_idx;

            all_mask[word_idx] |= mask;
            if m.win {
                win_mask[word_idx] |= mask;
                wins += 1;
            }

            // Dedup items in match
            let mut seen_items = std::collections::HashSet::new();
            for &item_id in &m.items {
                if !seen_items.insert(item_id) {
                    continue;
                }
                item_bitsets
                    .entry(item_id)
                    .or_insert_with(|| vec![0u64; num_words])[word_idx] |= mask;
                let counter = item_counts.entry(item_id).or_insert((0, 0));
                counter.0 += 1;
                if m.win {
                    counter.1 += 1;
                }
            }

            // Dedup augments in match
            let mut seen_augs = std::collections::HashSet::new();
            for aug_id in &m.augments {
                if !seen_augs.insert(aug_id.clone()) {
                    continue;
                }
                augment_bitsets
                    .entry(aug_id.clone())
                    .or_insert_with(|| vec![0u64; num_words])[word_idx] |= mask;
                let counter = augment_counts.entry(aug_id.clone()).or_insert((0, 0));
                counter.0 += 1;
                if m.win {
                    counter.1 += 1;
                }
            }
        }

        let win_rate = if total > 0 {
            wins as f64 / total as f64
        } else {
            0.50
        };

        Self {
            champion_id,
            total_matches: total,
            champion_wins: wins,
            champion_win_rate: win_rate,
            win_mask,
            all_mask,
            item_bitsets,
            augment_bitsets,
            item_counts,
            augment_counts,
        }
    }
}

#[inline]
pub fn bitset_count_ones(b: &[u64]) -> usize {
    b.iter().map(|w| w.count_ones() as usize).sum()
}

#[inline]
pub fn bitset_count_wins(b: &[u64], win_mask: &[u64]) -> usize {
    b.iter()
        .zip(win_mask.iter())
        .map(|(w, win)| (w & win).count_ones() as usize)
        .sum()
}

#[inline]
pub fn bitset_and(a: &[u64], b: &[u64]) -> Vec<u64> {
    a.iter().zip(b.iter()).map(|(x, y)| x & y).collect()
}

pub fn compute_build_recommendation_from_index(
    index: &ChampionTransactionIndex,
    request: BuildEngineRequest,
    load_time_ms: f64,
    cache_hit: bool,
) -> Result<BuildRecommendationResult, String> {
    let compute_start = Instant::now();

    if index.total_matches == 0 {
        let compute_ms = compute_start.elapsed().as_secs_f64() * 1000.0;
        return Ok(BuildRecommendationResult {
            champion_id: request.champion_id,
            active_combos: Vec::new(),
            next_items: Vec::new(),
            recommended_boots: None,
            completed_build: Vec::new(),
            alternative_items: Vec::new(),
            stats: EnginePerformanceStats {
                load_time_ms,
                compute_time_ms: compute_ms,
                total_time_ms: load_time_ms + compute_ms,
                match_count: 0,
                cache_hit,
            },
        });
    }

    let owned_items: Vec<u32> = request
        .owned_items
        .into_iter()
        .filter(|&id| crate::build_engine::item_catalog::is_valid_build_item(id))
        .collect();
    let owned_augments: Vec<String> = request
        .owned_augments
        .into_iter()
        .filter(|a| !a.trim().is_empty())
        .collect();

    let owned_legendaries: Vec<u32> = owned_items
        .iter()
        .copied()
        .filter(|&id| crate::build_engine::item_catalog::is_legendary_item(id))
        .collect();

    // 1. Real-time combo matching (item-item and item-augment) - strictly legendary items
    let mut active_combos = Vec::new();
    let m_combo = 8.0f64;

    // Item-Item combos
    for i in 0..owned_legendaries.len() {
        for j in (i + 1)..owned_legendaries.len() {
            let item_a = owned_legendaries[i];
            let item_b = owned_legendaries[j];
            if let (Some(b_a), Some(b_b)) = (
                index.item_bitsets.get(&item_a),
                index.item_bitsets.get(&item_b),
            ) {
                let intersect = bitset_and(b_a, b_b);
                let sample_size = bitset_count_ones(&intersect);
                if sample_size >= 1 {
                    let wins = bitset_count_wins(&intersect, &index.win_mask);
                    let shrunk_wr = (wins as f64 + m_combo * index.champion_win_rate)
                        / (sample_size as f64 + m_combo);
                    let wpa = shrunk_wr - index.champion_win_rate;
                    active_combos.push(ActiveComboSynergy {
                        combo_type: "item-item".to_string(),
                        item_ids: vec![item_a.min(item_b), item_a.max(item_b)],
                        augment_ids: Vec::new(),
                        label: format!("Item {} + Item {}", item_a, item_b),
                        win_rate: shrunk_wr,
                        wpa,
                        sample_size,
                    });
                }
            }
        }
    }

    // Item-Augment combos
    for &item_id in &owned_legendaries {
        for aug_id in &owned_augments {
            if let (Some(b_item), Some(b_aug)) = (
                index.item_bitsets.get(&item_id),
                index.augment_bitsets.get(aug_id),
            ) {
                let intersect = bitset_and(b_item, b_aug);
                let sample_size = bitset_count_ones(&intersect);
                if sample_size >= 1 {
                    let wins = bitset_count_wins(&intersect, &index.win_mask);
                    let shrunk_wr = (wins as f64 + m_combo * index.champion_win_rate)
                        / (sample_size as f64 + m_combo);
                    let wpa = shrunk_wr - index.champion_win_rate;
                    active_combos.push(ActiveComboSynergy {
                        combo_type: "item-augment".to_string(),
                        item_ids: vec![item_id],
                        augment_ids: vec![aug_id.clone()],
                        label: format!("Item {} + Augment {}", item_id, aug_id),
                        win_rate: shrunk_wr,
                        wpa,
                        sample_size,
                    });
                }
            }
        }
    }

    active_combos.sort_by(|a, b| b.wpa.partial_cmp(&a.wpa).unwrap_or(std::cmp::Ordering::Equal));

    // 2. Candidate items evaluation
    // ponytail: pairwise synergy lift instead of full-subgraph frequent itemset tree; upgrade to FP-Growth when dataset exceeds 1M matches.
    let m_item = 20.0f64;
    let mut candidate_scores: Vec<(u32, f64, f64, usize, bool, String)> = Vec::new();

    for (&cand_id, &(cand_games, cand_wins)) in &index.item_counts {
        if owned_items.contains(&cand_id)
            || !crate::build_engine::item_catalog::is_valid_build_item(cand_id)
        {
            continue;
        }

        // If this is an augment reward item (e.g. Wooglet's, Spatula, Ultra Hydra, Void Immolation),
        // only evaluate/recommend it if the player actually has the corresponding augment.
        if crate::build_engine::item_catalog::is_augment_reward_item(cand_id)
            && !crate::build_engine::item_catalog::player_has_augment_for_item(cand_id, &owned_augments)
        {
            continue;
        }

        let is_boot = crate::build_engine::item_catalog::is_completed_boot(cand_id);
        let uncond_wr = (cand_wins as f64 + m_item * index.champion_win_rate)
            / (cand_games as f64 + m_item);

        let cand_bitset = match index.item_bitsets.get(&cand_id) {
            Some(b) => b,
            None => continue,
        };

        let mut lifts = Vec::new();
        let mut augment_synergy_count = 0usize;

        // Synergy with owned legendary items
        for &owned_item in &owned_legendaries {
            if let Some(b_owned) = index.item_bitsets.get(&owned_item) {
                let intersect = bitset_and(cand_bitset, b_owned);
                let joint_games = bitset_count_ones(&intersect);
                if joint_games >= 2 {
                    let joint_wins = bitset_count_wins(&intersect, &index.win_mask);
                    let joint_wr = joint_wins as f64 / joint_games as f64;
                    let (o_games, o_wins) = index
                        .item_counts
                        .get(&owned_item)
                        .copied()
                        .unwrap_or((1, 0));
                    let o_wr = o_wins as f64 / o_games.max(1) as f64;
                    lifts.push(joint_wr - o_wr);
                }
            }
        }

        // Synergy with owned augments
        for owned_aug in &owned_augments {
            if let Some(b_aug) = index.augment_bitsets.get(owned_aug) {
                let intersect = bitset_and(cand_bitset, b_aug);
                let joint_games = bitset_count_ones(&intersect);
                if joint_games >= 2 {
                    let joint_wins = bitset_count_wins(&intersect, &index.win_mask);
                    let joint_wr = joint_wins as f64 / joint_games as f64;
                    let (a_games, a_wins) = index
                        .augment_counts
                        .get(owned_aug)
                        .copied()
                        .unwrap_or((1, 0));
                    let a_wr = a_wins as f64 / a_games.max(1) as f64;
                    let lift = joint_wr - a_wr;
                    lifts.push(lift);
                    if lift > 0.01 {
                        augment_synergy_count += 1;
                    }
                }
            }
        }

        // Context-aware pick rate threshold:
        // When the user has 0 or 1 augment/item (early phase), filter out low pick-rate anomalies
        // (< 5% pick rate or < 8 matches) to prevent skewed low-sample high-winrate outliers.
        // Once the player has >= 2 augments or items, allow niche items IF they have a positive
        // synergy lift with the player's active augments/items.
        let has_deep_context = (owned_augments.len() + owned_legendaries.len()) >= 2;
        let pick_rate = if index.total_matches > 0 {
            cand_games as f64 / index.total_matches as f64
        } else {
            1.0
        };

        if !is_boot && index.total_matches >= 20 {
            if !has_deep_context {
                // Early game: strictly require established pick rate and minimum volume
                if pick_rate < 0.05 || cand_games < 8 {
                    continue;
                }
            } else {
                // Later/deeper context: allow low pickrate only if it has positive synergy with current build
                let has_active_synergy = augment_synergy_count > 0 || lifts.iter().any(|&l| l > 0.02);
                if pick_rate < 0.02 && !has_active_synergy {
                    continue;
                }
            }
        }

        let avg_lift = if lifts.is_empty() {
            0.0
        } else {
            lifts.iter().sum::<f64>() / lifts.len() as f64
        };

        let score = uncond_wr + 0.4 * avg_lift;
        let reason = if avg_lift.abs() > 0.005 {
            format!(
                "+{:.1}% synergy lift across active build ({} games)",
                avg_lift * 100.0,
                cand_games
            )
        } else {
            format!(
                "{:.1}% Bayesian win rate ({} games)",
                uncond_wr * 100.0,
                cand_games
            )
        };

        candidate_scores.push((cand_id, score, avg_lift, cand_games, is_boot, reason));
    }

    // 3. Boots recommendation
    let owns_boots = owned_items.iter().any(|&id| crate::build_engine::item_catalog::is_completed_boot(id));
    let recommended_boots = if owns_boots {
        None
    } else {
        let mut boot_candidates: Vec<_> = candidate_scores
            .iter()
            .filter(|(_, _, _, _, is_boot, _)| *is_boot)
            .collect();
        boot_candidates.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        boot_candidates.first().map(|entry| {
            let (item_id, score, lift, sample_size, is_boot, ref reason) = **entry;
            ItemRecommendation {
                item_id,
                win_rate: score,
                lift,
                sample_size,
                is_boot,
                reason: reason.clone(),
            }
        })
    };

    // 4. Next 2 items recommendation (strictly legendary non-boots)
    let mut regular_candidates: Vec<_> = candidate_scores
        .iter()
        .filter(|(cand_id, _, _, _, is_boot, _)| !*is_boot && crate::build_engine::item_catalog::is_legendary_item(*cand_id))
        .collect();
    regular_candidates.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    let next_items: Vec<ItemRecommendation> = regular_candidates
        .iter()
        .take(2)
        .map(|entry| {
            let (item_id, score, lift, sample_size, is_boot, ref reason) = **entry;
            ItemRecommendation {
                item_id,
                win_rate: score,
                lift,
                sample_size,
                is_boot,
                reason: reason.clone(),
            }
        })
        .collect();

    // 5. Completed 5-item build path (legendaries only, boots separate in Recommended Boots)
    // ponytail: completing 5-item build greedily from ranked candidates; upgrade to dynamic programming knapsack when gold pacing is tracked.
    let mut completed_build_ids: Vec<u32> = owned_legendaries.clone();

    // Add next items (strictly legendary)
    for item in &next_items {
        if completed_build_ids.len() < 5 && !completed_build_ids.contains(&item.item_id) {
            completed_build_ids.push(item.item_id);
        }
    }

    // Fill remaining up to 5 items from regular candidates (strictly legendary)
    for candidate in &regular_candidates {
        if completed_build_ids.len() >= 5 {
            break;
        }
        let item_id = candidate.0;
        if !completed_build_ids.contains(&item_id) {
            completed_build_ids.push(item_id);
        }
    }

    // Map completed_build_ids to ItemRecommendation with reasons and metrics
    let completed_build: Vec<ItemRecommendation> = completed_build_ids
        .iter()
        .map(|&item_id| {
            if let Some(cand) = candidate_scores.iter().find(|c| c.0 == item_id) {
                ItemRecommendation {
                    item_id,
                    win_rate: cand.1,
                    lift: cand.2,
                    sample_size: cand.3,
                    is_boot: cand.4,
                    reason: cand.5.clone(),
                }
            } else {
                let (games, wins) = index.item_counts.get(&item_id).copied().unwrap_or((0, 0));
                let wr = if games > 0 { wins as f64 / games as f64 } else { index.champion_win_rate };
                ItemRecommendation {
                    item_id,
                    win_rate: wr,
                    lift: 0.0,
                    sample_size: games,
                    is_boot: false,
                    reason: format!("{:.1}% win rate ({} games)", wr * 100.0, games),
                }
            }
        })
        .collect();

    // 6. Alternative items not included in the primary 5-item build path or next items
    let alternative_items: Vec<ItemRecommendation> = regular_candidates
        .iter()
        .filter(|entry| !completed_build_ids.contains(&entry.0))
        .take(8)
        .map(|entry| {
            let (item_id, score, lift, sample_size, is_boot, ref reason) = **entry;
            ItemRecommendation {
                item_id,
                win_rate: score,
                lift,
                sample_size,
                is_boot,
                reason: reason.clone(),
            }
        })
        .collect();

    let compute_ms = compute_start.elapsed().as_secs_f64() * 1000.0;

    Ok(BuildRecommendationResult {
        champion_id: request.champion_id,
        active_combos,
        next_items,
        recommended_boots,
        completed_build,
        alternative_items,
        stats: EnginePerformanceStats {
            load_time_ms,
            compute_time_ms: compute_ms,
            total_time_ms: load_time_ms + compute_ms,
            match_count: index.total_matches,
            cache_hit,
        },
    })
}

pub fn compute_build_recommendation(
    request: BuildEngineRequest,
) -> Result<BuildRecommendationResult, String> {
    let load_start = Instant::now();
    let (index, cache_hit) =
        crate::build_engine::cache::get_or_build_champion_index(request.champion_id)?;
    let load_ms = load_start.elapsed().as_secs_f64() * 1000.0;
    compute_build_recommendation_from_index(&index, request, load_ms, cache_hit)
}

#[derive(Debug, Clone, PartialEq)]
pub struct TopSynergyItemResult {
    pub item_id: u32,
    pub win_rate: f64,
    pub lift: f64,
    pub sample_size: usize,
}

pub fn find_top_legendary_items_for_augment(
    index: &ChampionTransactionIndex,
    augment_id_or_name: &str,
    limit: usize,
) -> Vec<TopSynergyItemResult> {
    if index.total_matches == 0 || augment_id_or_name.trim().is_empty() {
        return Vec::new();
    }

    let b_aug = index
        .augment_bitsets
        .get(augment_id_or_name)
        .or_else(|| {
            index.augment_bitsets.iter().find_map(|(k, v)| {
                if k.eq_ignore_ascii_case(augment_id_or_name) {
                    Some(v)
                } else {
                    None
                }
            })
        });

    let b_aug = match b_aug {
        Some(b) => b,
        None => return Vec::new(),
    };

    let aug_games = bitset_count_ones(b_aug);
    if aug_games == 0 {
        return Vec::new();
    }
    let aug_wins = bitset_count_wins(b_aug, &index.win_mask);
    let baseline_aug_wr = aug_wins as f64 / aug_games as f64;

    let m_item = 8.0f64;
    let mut candidates: Vec<TopSynergyItemResult> = Vec::new();

    for (&item_id, item_bitset) in &index.item_bitsets {
        if !crate::build_engine::item_catalog::is_legendary_item(item_id) {
            continue;
        }

        if crate::build_engine::item_catalog::is_augment_reward_item(item_id) {
            let aug_list = [augment_id_or_name.to_string()];
            if !crate::build_engine::item_catalog::player_has_augment_for_item(item_id, &aug_list) {
                continue;
            }
        }

        let intersect = bitset_and(item_bitset, b_aug);
        let sample_size = bitset_count_ones(&intersect);

        let (total_item_games, _) = index.item_counts.get(&item_id).copied().unwrap_or((0, 0));
        let item_pick_rate = if index.total_matches > 0 {
            total_item_games as f64 / index.total_matches as f64
        } else {
            1.0
        };

        if sample_size < 2 {
            continue;
        }

        if index.total_matches >= 20 && (item_pick_rate < 0.03 || total_item_games < 5) {
            continue;
        }

        let joint_wins = bitset_count_wins(&intersect, &index.win_mask);
        let bayesian_wr = (joint_wins as f64 + m_item * baseline_aug_wr) / (sample_size as f64 + m_item);
        let lift = bayesian_wr - baseline_aug_wr;

        candidates.push(TopSynergyItemResult {
            item_id,
            win_rate: bayesian_wr,
            lift,
            sample_size,
        });
    }

    candidates.sort_by(|a, b| {
        b.lift
            .partial_cmp(&a.lift)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| b.sample_size.cmp(&a.sample_size))
    });

    candidates.truncate(limit);
    candidates
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::build_engine::data_loader::RawParticipantMatch;

    #[test]
    fn test_bitset_intersection_and_recommendation() {
        let mut matches = Vec::new();
        for i in 0..100 {
            let win = i % 2 == 0;
            let mut items = Vec::new();
            let mut augments = Vec::new();

            // Item 3006 (Berserker boots)
            if i % 3 == 0 {
                items.push(3006);
            }
            // Item 3031 (Infinity Edge)
            if i < 70 {
                items.push(3031);
            }
            // Item 3046 (Phantom Dancer)
            if i < 50 {
                items.push(3046);
            }
            // Augment "aug_1"
            if i < 60 {
                augments.push("aug_1".to_string());
            }

            matches.push(RawParticipantMatch {
                win,
                items,
                augments,
            });
        }

        let mut index = ChampionTransactionIndex::from_matches(999, &matches);
        assert_eq!(index.total_matches, 100);
        assert_eq!(index.champion_wins, 50);

        // Bitset intersection: 3031 & 3046 intersect in 50 matches (i < 50)
        let b1 = &index.item_bitsets[&3031];
        let b2 = &index.item_bitsets[&3046];
        let intersection = bitset_and(b1, b2);
        assert_eq!(bitset_count_ones(&intersection), 50);

        // Compute recommendation when player owns 3031 and aug_1
        let req = BuildEngineRequest {
            id: 1,
            champion_id: 999,
            owned_items: vec![3031],
            owned_augments: vec!["aug_1".to_string()],
        };

        let result = compute_build_recommendation_from_index(&index, req, 0.1, false).unwrap();

        // 3031 + aug_1 active combo
        assert!(!result.active_combos.is_empty());
        assert_eq!(result.active_combos[0].combo_type, "item-augment");

        // Player does not own boots, so boots should be recommended (3006)
        assert!(result.recommended_boots.is_some());
        assert_eq!(result.recommended_boots.unwrap().item_id, 3006);

        // Next items should recommend 3046
        assert!(!result.next_items.is_empty());
        assert_eq!(result.next_items[0].item_id, 3046);

        // Completed build contains 3031, 3046 (no boots)
        assert!(!result.completed_build.iter().any(|i| i.item_id == 3006));
        assert!(result.completed_build.iter().any(|i| i.item_id == 3031));
        assert!(result.completed_build.iter().any(|i| i.item_id == 3046));

        // When player already owns boots (3006), recommended_boots must be None
        let req_with_boots = BuildEngineRequest {
            id: 2,
            champion_id: 999,
            owned_items: vec![3006, 3031, 1058], // 1058 is Needlessly Large Rod (component)
            owned_augments: vec!["aug_1".to_string()],
        };
        let result2 =
            compute_build_recommendation_from_index(&index, req_with_boots, 0.1, false).unwrap();
        assert!(result2.recommended_boots.is_none());
        // Component 1058 must NOT be in completed_build
        assert!(!result2.completed_build.iter().any(|i| i.item_id == 1058));
        // Next items must only contain legendary items
        for item in &result2.next_items {
            assert!(crate::build_engine::item_catalog::is_legendary_item(item.item_id));
        }

        // Test augment reward exclusion
        // Insert Wooglet's Witchcap (228002) into item counts and bitsets
        index.item_counts.insert(228002, (2, 2));
        index.item_bitsets.insert(228002, vec![0b11]);

        // Without augment 1156, Wooglet should not be recommended
        let req_no_aug = BuildEngineRequest {
            id: 3,
            champion_id: 999,
            owned_items: vec![3031],
            owned_augments: vec![],
        };
        let rec_without_aug = compute_build_recommendation_from_index(&index, req_no_aug, 0.1, false).unwrap();
        assert!(!rec_without_aug.next_items.iter().any(|i| i.item_id == 228002));
        assert!(!rec_without_aug.completed_build.iter().any(|i| i.item_id == 228002));

        // With augment 1156, Wooglet is allowed
        let req_with_aug = BuildEngineRequest {
            id: 4,
            champion_id: 999,
            owned_items: vec![3031],
            owned_augments: vec!["1156".to_string()],
        };
        let rec_with_aug = compute_build_recommendation_from_index(&index, req_with_aug, 0.1, false).unwrap();
        assert!(rec_with_aug.next_items.iter().any(|i| i.item_id == 228002) || rec_with_aug.completed_build.iter().any(|i| i.item_id == 228002));

        // Test find_top_legendary_items_for_augment
        let top_items = find_top_legendary_items_for_augment(&index, "aug_1", 2);
        assert!(!top_items.is_empty());
        assert!(top_items.iter().any(|ti| ti.item_id == 3031 || ti.item_id == 3046));
        // Boots 3006 must NEVER be included
        assert!(!top_items.iter().any(|ti| ti.item_id == 3006));
    }
}
