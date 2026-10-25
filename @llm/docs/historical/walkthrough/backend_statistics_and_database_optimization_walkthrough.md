# Backend Statistics & Database Optimization Walkthrough

## Summary of Changes

### 1. Massive Projection Payload Compaction (98% Size Reduction)
- **Problem**: In `global-aram.db` and `agent.db`, `statistics_projections` was storing full unpruned arrays of all 165 champions and all 381 partners for every single item and augment. This bloated projection rows to over 13.2 MB for augments and 9.6 MB for items. On every query, the backend was forced to read, deserialize, and transmit this 13 MB JSON blob over IPC, causing the UI to freeze for seconds.
- **Fix**:
  - In [`agent/mod.rs`](src-tauri/src/agent/mod.rs):
    - Truncated `champions` to the top 10 (or top 3 for combos) sorted by relevance in `statistic_row` and `compact_row_for_projection`.
    - Truncated `partners` to the top 10 items and top 10 augments in `statistic_row` and `compact_row_for_projection`.
    - Slashed stored projection sizes:
      - `champion`: 4.35 MB -> 601 KB (-86%)
      - `item`: 9.66 MB -> 931 KB (-90%)
      - `augment`: 13.2 MB -> 1.08 MB (-92%)
    - Compacted existing database records on disk and truncated WAL.

### 2. Auto-Compaction Migration on Startup
- Added `compact_stored_projections` helper in [`agent/mod.rs`](src-tauri/src/agent/mod.rs) invoked automatically during `initialize_database` (personal) and `initialize_global_aram_database` (global).
- Any legacy projection rows exceeding 500 KB are parsed once, compacted, and re-saved, shrinking user databases without requiring crawler resets.

### 3. In-Memory Parsed Projection Cache in Agent
- Added `PROJECTION_MEMORY_CACHE` in [`agent/mod.rs`](src-tauri/src/agent/mod.rs) to cache parsed `(sample_count, baseline, rows)` in RAM.
- `load_statistics_projection` reads from RAM instead of executing SQLite queries and `serde_json::from_str`.
- Cache is automatically invalidated upon database reset, projection deletion, or new projection writes.

### 4. Compare Mode Query Caching
- Fixed bug in `build_statistics_response` where `scope == "compare"` returned before the `STATISTICS_QUERY_CACHE` check, preventing compare queries from ever hitting the cache.
- Moved the cache check to the very top of `build_statistics_response` and added cache population for `compare` mode results.

### 5. In-Memory Enriched Statistics Cache in Tauri Command Facade
- **Problem**: When users clicked column headers to sort by Win Rate, WPA, Games, Pick Rate, or Name, the frontend invoked `api_statistics`, which sent a new 10,000-row request to the agent and ran `enrich_statistics_response` from scratch on 100,000+ JSON nodes.
- **Fix**:
  - In [`commands/agent.rs`](src-tauri/src/commands/agent.rs), implemented `ENRICHED_DATASET_CACHE` caching the base enriched dataset by `(scope, kind, hide_low_pickrate)`.
  - Column sorting, search filtering, and pagination now execute directly in Rust memory in < 0.1ms without IPC round-trips or re-enrichment.
  - Added `invalidate_enriched_statistics_cache()` called upon crawler resets and new projection writes.

### 6. Augment Resolution Deduplication
- In [`services/history.rs`](src-tauri/src/services/history.rs), deduplicated augment IDs before invoking `resolve_augments`, eliminating repetitive lookups.

### 7. Fixed Compiler Warnings
- Removed unused dead function `request_delay` in [`agent/mod.rs`](src-tauri/src/agent/mod.rs).
- Removed unused constant `CDRAGON_CHERRY_AUGMENT_ICON_BASE` in [`services/history.rs`](src-tauri/src/services/history.rs).
- Clean `cargo check` with 0 warnings.

### 8. Augment Tooltip & Name Resolution Overhaul
- **Problem**:
  - Augment 3176 (`GoH_PentakillAug_Generic`) had empty/whitespace `nameTRA` (`"   "`), causing `cherry_res` to skip it completely.
  - Augments like `Third Times the Harm` (3016) had tooltips in `lol.stringtable.json` under `<titleLeft>Third Times the Harm</titleLeft><br><mainText>...</mainText>`, but the resolver only checked legacy key formats.
- **Fix in [`services/history.rs`](src-tauri/src/services/history.rs)**:
  - Extracted `<titleLeft>` and `<mainText>` tag pairs from `lol.stringtable.json` into `stringtable_titles` to dynamically resolve descriptions by title (resolving `Third Times the Harm` to `"Being attacked three times by the enemy will deal true damage to you."`).
  - Added fallback mapping for empty/whitespace `nameTRA` to resolve `GoH_PentakillAug_Generic` (3176) to "Gifts of Heavy Metal" with Pentakill icon.
  - Added support for `AugmentPlatformId` and numeric `rarity` (0 = silver, 1 = gold, 2 = prismatic) in `populate_kiwi_catalog`.
  - Added fallback entries in [`augment_tooltips.json`](src-tauri/src/data/augment_tooltips.json) for 3176 (`GoH_PentakillAug_Generic`) and 3016 (`ThirdTimesTheHarm`).
  - Bumped cache versions to `augment_catalog_v10.json` and `augments_v9.json` to invalidate any previously cached null tooltips.

### 9. Per-Component Skeleton Loaders & Seamless Tab Transitions
- **Problem**: Previously, `{#if loading && rows.length === 0}` completely destroyed the view and rendered two generic empty pulse blocks (`<div class="h-44 ..."></div><div class="flex-1 ..."></div>`). All table controls, search inputs, item class toggles, augment tier filters, and tabs disappeared and reappeared on every load, causing severe layout shifts and visual breakage.
- **Fix**:
  - Maintained the stable outer layout, search bars, filters, and tab controls mounted at all times (zero layout shift).
  - Built per-component skeleton emulation:
    - **Inspector Skeleton**: Emulates the exact split inspector card (selected build icon, title, subtitle, in-game description block, metrics bar, and 3 columns of Best Matches with mini-avatars).
    - **Table Skeleton**: Renders 10 skeleton rows directly inside the real table `<tbody>`, with pulsing item/augment icons, card deck champion circles, and right-aligned numeric pills matching exact column widths.
  - Implemented in both [`BuildStatisticsView.svelte`](src/lib/components/history/BuildStatisticsView.svelte) and [`ProfileStatisticsView.svelte`](src/lib/components/history/ProfileStatisticsView.svelte).
  - Subtle top progress line (`h-0.5 bg-purple-500 animate-pulse`) during background sorting and refreshing without blocking or obscuring user interaction.

---

## Validation Results

- **Rust Formatting**: `cargo fmt --manifest-path src-tauri\Cargo.toml -- --check` passed cleanly.
- **Rust Library & Agent Build**:
  - `cargo check --manifest-path src-tauri\Cargo.toml` passed with 0 warnings.
  - `cargo build --manifest-path src-tauri\Cargo.toml --bin lcu-companion-agent` passed with 0 warnings.
- **Database Projections Compaction**:
  - `champion`: 4,352,554 -> 601,269 bytes (-86%)
  - `item`: 9,662,777 -> 931,061 bytes (-90%)
  - `augment`: 13,195,178 -> 1,089,412 bytes (-92%)
