# ARAM Mayhem Build Intelligence Platform & Multi-Agent Execution Plan

## 1. Summary
Overhaul the ARAM Mayhem build recommender into a real-time, augment-adaptive build guide and in-game shop assistant. The work is split into 4 isolated execution phases, each assigned a self-contained agent prompt to prevent context degradation.

---

## 2. Architecture & Subsystems

```text
┌────────────────────────────────────────────────────────┐
│ Phase 1: Knowledge Base (.llm.txt)                     │
│ League Wiki + Hextech Live Client Data Specs           │
└──────────────────────────┬─────────────────────────────┘
                           ▼
┌────────────────────────────────────────────────────────┐
│ Phase 2: Live In-Game Client Connector (Port 2999)      │
│ Real-time gold, inventory, stats, and item events      │
└──────────────────────────┬─────────────────────────────┘
                           ▼
┌────────────────────────────────────────────────────────┐
│ Phase 3: Augment-Aware Dynamic Build Engine (Rust)     │
│ Stat-conversion math, stage pathing, synergy scoring   │
└──────────────────────────┬─────────────────────────────┘
                           ▼
┌────────────────────────────────────────────────────────┐
│ Phase 4: Svelte 5 Dynamic Build Assistant UI           │
│ Live next-item recommendation, build path tree, tooltips│
└────────────────────────────────────────────────────────┘
```

---

## 3. Subsystem Breakdown

### Subsystem 1: Knowledge Base Generation (`@llm/docs/mayhem/`)
- Extract and structure wiki content into high-density `.llm.txt` files:
  - `@llm/docs/mayhem/RULES.llm.txt`: Mode overrides, overcap crit conversion, 5.0 AS cap, combo breaker, stat anvils, headliners.
  - `@llm/docs/mayhem/AUGMENTS.llm.txt`: All augments categorized by tier (Silver/Gold/Prismatic), triggers, stat scalings, and synergy tags (`ap_on_hit`, `ad_to_ap`, `crit_haste`, `tank_scale`, `omnivamp`).
  - `@llm/docs/mayhem/ITEMS.llm.txt`: Mayhem-exclusive items (Atma's, Rite of Ruin, Sword of Blossoming Dawn, Anvils, Quest rewards) and exclusive item locks (Hydra, Lifeline, Last Whisper).
  - `@llm/docs/mayhem/LIVE_CLIENT_API.llm.txt`: Specification for `https://127.0.0.1:2999/liveclientdata/allgamedata`.

### Subsystem 2: Live Client Data Engine (`src-tauri/src/services/live_game.rs`)
- Background poll service (1.5s interval during active match):
  - Connect to `https://127.0.0.1:2999/liveclientdata/allgamedata` with TLS certificate verification disabled.
  - Track player inventory slots, current gold, current champion level, stats (AP, AD, AS, Crit), and purchase history.
  - Expose Tauri command `api_get_live_game_state` and push Tauri event `live:state-updated`.

### Subsystem 3: Dynamic Build Recommendation Engine (`src-tauri/src/services/history.rs`)
- Upgrade `compute_mayhem_champion_build` from static slot order to dynamic multi-factor recommender:
  - **Augment Shift Detection**: Check equipped augments for build warping tags (e.g. *Marksmage* -> prioritize Nashor's Tooth / Lich Bane / Rabadon's even on AD marksmen; *ADAPt* -> AD-to-AP conversion path; *Big Shot* -> AS-to-Crit conversion).
  - **Game Stage Pathing**:
    - Stage 1: Starter + Boots (0-5 min / Lv 3-6).
    - Stage 2: First Mythic/Core spike (Lv 7 augment synergy).
    - Stage 3: Mid-game counter & synergy continuation (Lv 11 augment).
    - Stage 4: Capstone/Late-game situational items & anvil purchases (Lv 15+).
  - **Budget Affordability**: Given live player gold, suggest:
    - Best immediate component or full item purchaseable right now.
    - Next item target to save for.

### Subsystem 4: Frontend Build Page Overhaul (`src/lib/components/history/`)
- Adapt `MayhemChampSelectView.svelte` and `BuildStatisticsView.svelte`:
  - **Next Item Assistant Widget**: Highlights recommended purchase based on current gold and inventory.
  - **Interactive Augment Impact Inspector**: Shows how selecting an augment changes item win rates and recommendations in real time.
  - **Wiki Data Explorer Tabs**: Visual browse view for Mayhem augments and items with rich popovers.

---

## 4. Multi-Agent Phased Prompts

### Agent Prompt 1: Knowledge Base Creation
```markdown
You are an expert League of Legends data engineer.
Task: Create the ARAM Mayhem knowledge base files under `@llm/docs/mayhem/`.
1. Use the pre-scraped wiki files in `C:\Users\lucas\AppData\Local\Temp\opencode\aram_mayhem_wiki_extracted.txt` and `aram_mayhem_augments_extracted.txt`.
2. Generate:
   - `@llm/docs/mayhem/RULES.llm.txt`
   - `@llm/docs/mayhem/AUGMENTS.llm.txt`
   - `@llm/docs/mayhem/ITEMS.llm.txt`
   - `@llm/docs/mayhem/LIVE_CLIENT_API.llm.txt`
Format: Terse, token-dense, categorized by scaling and mechanics.
Verification: Check that all 4 files exist and contain accurate augment scaling and item tags.
```

### Agent Prompt 2: Live In-Game Client Connector
```markdown
You are a Rust backend systems engineer.
Task: Implement the Live Client Data API service in `src-tauri/src/services/live_game.rs`.
1. Implement polling to `https://127.0.0.1:2999/liveclientdata/allgamedata` with TLS certificate validation skipped.
2. Parse active player gold, champion stats, current items, and game time.
3. Expose Tauri command `api_get_live_game_state` in `src-tauri/src/commands/champ_select.rs` and register in `src-tauri/src/lib.rs`.
4. Emit `live:game-state` event when in-game status changes.
Verification: Run `cargo check --manifest-path src-tauri/Cargo.toml`.
```

### Agent Prompt 3: Dynamic Augment-Aware Build Recommender
```markdown
You are a mathematical algorithm and game systems developer.
Task: Overhaul the Mayhem build recommendation algorithm in `src-tauri/src/services/history.rs`.
1. Ingest augment tags from `@llm/docs/mayhem/AUGMENTS.llm.txt`.
2. Implement augment build warping (e.g. Marksmage AP shift, ADAPt, Big Shot, Escalating Chaos).
3. Compute stage-based next-item recommendations given current inventory items and gold.
4. Add unit tests verifying:
   - Marksmage converts ADC recommended item pool to AP/on-hit.
   - Situational items correctly flag counter-picks (anti-heal, anti-shield).
Verification: Run `cargo test --manifest-path src-tauri/Cargo.toml`.
```

### Agent Prompt 4: Build Page UI & Live Assistant
```markdown
You are a Svelte 5 frontend developer.
Task: Enhance `MayhemChampSelectView.svelte` and create the live Next-Item Assistant.
1. Connect to `api_get_live_game_state` and listen for `live:game-state`.
2. Render "Live Purchase Advisor" when in match showing affordable items and next power-spike target.
3. Provide quick-toggle augment simulation to preview how augments shift recommendations.
4. Add a "Mayhem Encyclopedia" modal/tab using Flowbite Svelte components and GameEntityPopover.
Verification: Run `npx svelte-check --tsconfig ./tsconfig.json` and `cargo check`.
```

---

## 5. Verification & Test Plan
1. **Knowledge Integrity**: Ensure `@llm/docs/mayhem/*.llm.txt` accurately catalog all ~100 augments and Mayhem-exclusive items.
2. **Backend Compilation & Unit Tests**: `cargo test --manifest-path src-tauri/Cargo.toml` verifying build archetype generation with augment shifts.
3. **Frontend Type Checking**: `npx svelte-check --tsconfig ./tsconfig.json` with 0 errors.
4. **Live API Handling**: Fallback gracefully when game client is not running (`connection_refused` -> status `NotInGame`).
