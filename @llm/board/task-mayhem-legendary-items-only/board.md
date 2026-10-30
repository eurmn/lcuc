# Sub-board: Filter Mayhem Build Engine to Legendary Items & Completed Boots Only (`task-mayhem-legendary-items-only`)

## Objective
Filter the empirical bitset build recommendation engine (`lcu-build-engine`) so that components (and sub-legendary items) are completely ignored. Only legendary items (and completed boots) participate in transactions, bitset indices, combo mining, candidate evaluation, next-item recommendations, and completed build paths. Once verified, bundle the app and provide the final setup installer.

## Problem Context
- Players frequently finish games with unfinished components in inventory (e.g. Tear of the Goddess 3070, Amp Tome 1052, Pickaxe 1037, B.F. Sword 1038, Vamp Scepter 1053, Kindlegem 3067, Bramble Vest 3076, Phage 3044).
- The bitset miner was treating any item with price >= 400 as an item candidate, which caused components to appear in active combos, synergy lifts, `next_items` recommendations, and the completed 6-item build.
- User requirement: "keep only legendary items as items in the algorithm ok. ignore components entirely".

## Classification & Catalog
- Static bundle generated at `src-tauri/src/data/legendary_items.json`:
  - 439 legendary items (`priceTotal >= 2000 && to.is_empty()` or `rarityLegendary` / `rarityMythic` or ARAM/Arena mode-specific legendaries / Seraph's / Muramana / Fimbulwinter).
  - 25 completed boots (`categories.contains("boots") && to.is_empty() && id != 1001 && id != 2422`).
  - 121 components (`to.len() > 0` or price < 2000 non-legendary non-boots).

## Implementation Plan
1. Create `src-tauri/src/build_engine/item_catalog.rs`:
   - Fast lookup using compile-time sets / lazy hashsets parsed from `include_str!("../data/legendary_items.json")`.
   - Helper functions: `is_legendary_item(item_id: u32) -> bool`, `is_completed_boot(item_id: u32) -> bool`, `is_valid_build_item(item_id: u32) -> bool`.
2. Update `src-tauri/src/build_engine/data_loader.rs`:
   - In `parse_items`, retain ONLY items that satisfy `is_valid_build_item(id)` (legendaries and completed boots).
3. Update `src-tauri/src/build_engine/bitset_miner.rs`:
   - Combo mining: only mine item-item and item-augment combos where items are strictly legendary.
   - Recommended boots: select exclusively from completed boots.
   - Candidate ranking (`next_items`) & greedy completed build: candidates must strictly be legendary items (`is_legendary_item(id)`).
   - Ignore any owned items that are components from calculation.
4. Verify with Rust unit tests (`cargo test --lib build_engine`).
5. Run Tauri bundle build: `bun run tauri build` to generate the Windows setup executable.

---
### Progress Log
- `2026-10-29 20:30` [@orchestrator]: Sub-board initialized. Ready to implement.
