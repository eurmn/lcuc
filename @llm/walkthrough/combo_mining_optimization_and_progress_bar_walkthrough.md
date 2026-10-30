# Combo Mining Optimization & Progress Bar Walkthrough

## 1. Root Cause
- Missing `"combo"` row in `statistics_projections` forced on-demand calculation across 600,010 rows in `aram_participants`.
- Fixed `min_combo_games = 3` failed to prune candidate pairs on large datasets, causing a combinatorial explosion in $k=2..5$ tuple generation.
- `sort_key == "name"` gate prevented `MayhemCombosCard` (`sort_key: "wpa"`) from persisting computed projections to SQLite.

## 2. Backend Fixes (`src-tauri/src/agent/mod.rs`)
- Bounded combo fact query (`ORDER BY p.rowid DESC LIMIT 25000`).
- Dynamic Apriori support scaling (`min_combo_games` and `frequent_core_pairs` for $k=4, 5$).
- Unconditional persistence of `unfiltered_rows` (capped to top 1000 by WPA) into `statistics_projections`.
- 4-stage live progress reporting via `ACTIVE_STATS_OPERATION` and RAII `ActiveStatsOpGuard`.
- In-memory `STATISTICS_QUERY_CACHE` insertion on uncached query completion when `offset == 0 && rows_arr.len() < limit`.

## 3. Frontend Fixes (`src/lib/components/history/MayhemCombosCard.svelte`, `MayhemChampSelectView.svelte`, `App.svelte`)
- Live operation polling (350ms) and prop propagation.
- Dynamic header badge and 4-step progress bar panel (`1. Load Facts`, `2. Frequency`, `3. Synergy Pairs`, `4. Win Rate & WPA`).

## 4. Statistical Combo Diversity & Champion-Scoped Mining
- **Champion-Scoped Combo Mining**: Added `champion_id` filter to `read_statistics_facts` and `api_statistics`. If fact count `< 15`, falls back to global dataset to avoid empty sets on unplayed champions.
- **Support Floor & Mayhem Shapes**: Set `min_combo_games = 3` and Bayesian shrinkage `m_wr = 6.0`. Generates exactly 5 Mayhem item/augment shapes: `1A+1I`, `1A+2I`, `2A+1I`, `2A+2I`, `2A+3I`.
- **Log-IDF Ubiquity Penalty & Interaction Synergy Lift**: Replaced raw WPA sort with `synergy_score = (0.55 * adj_wpa + 0.45 * lift) * mean_spec`, suppressing ubiquitous item bloat and lifting true synergy pairs.
- **Component Diversity Cap**: Top-1000 backend projection pool enforces max 3 occurrences per item/augment component; `MayhemCombosCard.svelte` enforces presentation cap `MAX_PER_COMPONENT = 2` for diverse UI cards.
