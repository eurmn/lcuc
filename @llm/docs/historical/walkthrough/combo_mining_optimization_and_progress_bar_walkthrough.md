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
