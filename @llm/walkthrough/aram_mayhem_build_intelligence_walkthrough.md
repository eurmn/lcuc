# ARAM Mayhem Build Intelligence Platform Walkthrough

## Summary
Implemented the 4-phase ARAM Mayhem Build Intelligence Platform:
1. **Knowledge Base**: Mode overrides, augment tags, exclusive items, and live client API specs in `@llm/docs/mayhem/*.llm.txt`.
2. **Live Game Connector**: In-game client polling (`127.0.0.1:2999/liveclientdata/allgamedata`), TLS bypass, `live:game-state` and `live:state-updated` event broadcasts, and `api_get_live_game_state` command.
3. **Dynamic Build Engine**: Stat-conversion math & augment build warping (`Marksmage`, `ADAPt`, `Big Shot`, `Apex Inventor`, `Vulnerability`, `Goliath`), 4-stage build pathing, budget affordability advisor (`api_get_live_build_advice`), situational counter-pick classification (`Anti-Heal`, `Anti-Shield`, `Armor Penetration`, etc.), and unit tests.
4. **Svelte 5 Assistant UI**: `LivePurchaseAdvisor.svelte`, `StagePathingTree.svelte`, `MayhemEncyclopediaModal.svelte`, 1-click augment simulation quick-toggles, and rich popovers across champ select and statistics views.

---

## Changes

### 1. Knowledge Base (`@llm/docs/mayhem/`)
- `RULES.llm.txt`: Overcap crit adaptive conversion (+0.45 AD / +0.75 AP), 5.0 AS cap, combo breaker CC cleanse + 3s immunity, innate Presence of Mind, headliner bands (Command Line, Cafe Cuties, Heartsteel, Space Groove, The Bee Band, True Damage, K/DA, Pentakill).
- `AUGMENTS.llm.txt`: Catalog of build-warping augments and archetype synergy tags (`ap_on_hit`, `ad_to_ap`, `ap_to_ad`, `as_to_crit`, `item_haste`, `dot_crit`, `tank_scale`, `melee_transmutation`).
- `ITEMS.llm.txt`: Mayhem legendaries (Atma's Reckoning, Rite of Ruin, Sword of Blossoming Dawn), augment quest items (Wooglet's Witchcap, Golden Spatula, Ultra Hydra), and exclusivity lock groups.
- `LIVE_CLIENT_API.llm.txt`: HTTPS port 2999 endpoint specification and payload structure.

### 2. Backend Rust Core (`src-tauri/`)
- `src-tauri/src/services/live_game.rs`:
  - Added background polling task (`start_live_game_polling`) emitting `live:game-state` and `live:state-updated`.
  - Integrated `fetch_live_game_state` with 1500ms timeout and disabled certificate validation.
- `src-tauri/src/models/history.rs`:
  - Added `situational_tag?: Option<String>` to `MayhemBuildItem`.
  - Added `MayhemStageItem` and `MayhemLiveAdvice` domain models.
  - Added `stages: Vec<MayhemStageItem>` to `MayhemBuildStrategy`.
- `src-tauri/src/services/history.rs`:
  - `get_situational_tag`: Categorizes counter-picks into Anti-Heal, Anti-Shield, Armor/Magic Penetration, Magic Resist, Armor/Anti-Burst, and Cleanse/QSS.
  - `get_item_augment_warp_bonus`: Applies stat-conversion multipliers to item scores when augments like Marksmage or ADAPt are present.
  - `build_augment_warped_archetype`: Injects mathematically optimal AP On-Hit, AD-to-AP, or AS-to-Crit archetypes into strategy recommendations.
  - `build_stage_pathing`: Generates 4-stage progression tree (Starter+Boots, First Core Spike, Synergy Continuation, Capstone & Situational).
  - `compute_live_build_advice`: Calculates stage, next target item, gold needed, and affordable immediate purchases.
  - Added unit tests: `test_situational_items_counter_pick_flags`, `test_marksmage_converts_adc_item_pool_to_ap_on_hit`, and `test_budget_affordability_live_advice`.
- `src-tauri/src/commands/champ_select.rs`:
  - Added `api_get_live_build_advice` Tauri command.
- `src-tauri/src/lib.rs`:
  - Registered `api_get_live_game_state` and `api_get_live_build_advice`.
  - Started `start_live_game_polling` on app launch.

### 3. Frontend Svelte 5 (`src/lib/`)
- `src/lib/types/champ_select.ts` & `src/lib/api/champ_select.ts`:
  - Added `LiveGameState`, `LivePlayerItem`, `MayhemStageItem`, `MayhemLiveAdvice` typings and wrappers.
- `src/lib/components/history/LivePurchaseAdvisor.svelte`:
  - In-game widget showing active match timer, current gold, stage badge, next item target with progress bar, and immediate affordable items.
- `src/lib/components/history/StagePathingTree.svelte`:
  - 4-column visual build path breakdown with gold prices, popovers, and counter-pick badges.
- `src/lib/components/history/MayhemEncyclopediaModal.svelte`:
  - Tabbed browser for mode overrides, exclusive items & locks, and build-warping augments with 1-click simulation.
- `src/lib/components/history/MayhemChampSelectView.svelte`:
  - Added real-time event listeners and polling for `live:game-state`.
  - Added quick-toggle augment simulation bar (`Marksmage`, `ADAPt`, `Big Shot`, `Apex Inventor`, etc.).
  - Embedded `LivePurchaseAdvisor`, `StagePathingTree`, and `MayhemEncyclopediaModal`.
- `src/lib/components/history/BuildStatisticsView.svelte`:
  - Added Encyclopedia button in the build performance toolbar opening the modal.
