# Walkthrough: Release v0.2.0 Features & Bug Fixes

**Handoff Date:** 2026-10-26  
**Milestone:** v0.2.0 Feature Complete  
**Scope:** ARAM Mayhem Champion Select, Global Crawler Controls, Rich Tooltips & Logs, System Tray Status, Build Statistics Overhaul

---

## 1. Summary of Changes

### A. Champion Selection Sync & Pinning
- **Selection Synchronization**: Connected `MayhemChampSelectView.svelte` to the global champion state in `App.svelte` via `selectedChampion` and `onSelectChampion={handleSelectChampionFilter}`.
- **Top Bar Sync**: Selection in the Mayhem Champ Select carousel immediately syncs with `TopNavBar.svelte` and persists into `localStorage` across page navigations and restarts.
- **Pinned Selected Champion First**: Added `pinSelectedFirst` property to `ChampionCarousel.svelte`. When a champion is selected, their avatar is automatically pinned to index 0, with a glowing selection border, while filtering out duplicates from the rest of the list.

### B. Mayhem Statistics & Associated Entities Overhaul
- **Poro-Snax Filtering**: Excluded Poro-Snax item IDs (`2050`, `2052`) from both backend partner facts accumulation (`src-tauri/src/agent/mod.rs`) and frontend inspector filtering (`BuildStatisticsView.svelte`).
- **Tri-Sort Selector for Associated Items & Augments**: Added dynamic sorting tabs for `Most Picked`, `Highest Win-Rate`, and `Highest WPA` in `BuildStatisticsView.svelte`.
- **Rich Popover Integration**: Integrated `GameEntityPopover.svelte` with unique hover targets for all associated items and augments.
- **Entity Limit Raised to 15**: Increased partner capacity in `src-tauri/src/agent/mod.rs` from 10 to 30, and expanded UI slice from 5 to 15 items and augments.

### C. Champion Build Refinements & Spell Pair Optimization
- **Smart Starter Item Omission**: In `src-tauri/src/services/history.rs`, if a champion's starter item pick rate is negligible (< 5%) or sample size is below threshold, `starter_item` is set to `None`, omitting the starter item box entirely.
- **Tooltip Template Parsing Fixes**:
  - `%i:scaleAP%`: Added inline icon resolver matching `TAG_DEFINITIONS[key]?.iconUrl` in `src/lib/utils/gameDescription.ts`.
  - `@Spell.Augment_EnchantingButterflies:MaxButterfly@`: Broadened regex across frontend and backend (`src-tauri/src/services/history.rs`) to parse namespaced cross-spell tokens with colons and dots (`:` and `.`).
- **Summoner Spell Pair Scoring**: Overhauled spell pair selection with Bayesian shrinkage ($M=40.0$), dynamic `min_pair_games` proportional to total games played, and an individual spell negative WPA filter ($> -1.5\%$). This prevents low-sample Mark (`-1.4%` WPA on Ashe) from displacing high-performing Flash / Barrier / Ghost.

### D. Global ARAM Crawler Controls & Progress Banner
- **ETA Calculation**: Integrated remaining match and rate calculations in `src-tauri/src/agent/mod.rs` to compute dynamic ETA (e.g. `2m 14s`, `1h 05m`), exposed via `crawler_eta_text`.
- **Universal Top Progress Banner**: Moved crawler progress banner out of Mayhem Statistics into the root layout in `App.svelte`, rendering seamlessly across `Builds`, `Mayhem`, and `Profile` views.
- **TitleBar Integration**:
  - Moved the pulsing **Active Operations** dot into `TitleBar.svelte` with transparent background, hover glow, and dropdown progress drawer.
  - Moved **Global Database Settings** cog into `TitleBar.svelte`, featuring crawler status, rate, ETA, worker pause/resume toggle, crawler stop button, game limit editor, and database reset with confirmation.

### E. Logs Modal & Real-Time Streaming
- **Log Engine Integration**: Added `init_app_handle` and bounded in-memory buffer (`read_recent_logs`) to `src-tauri/src/logger.rs`, broadcasting real-time `log_entry` events to the Tauri webview.
- **Logs Modal (`LogsModal.svelte`)**:
  - Terminal-style dark console opened directly from the TitleBar ("Logs" button).
  - Filter by log level (`ALL`, `INFO`, `WARN`, `ERROR`, `DEBUG`).
  - Search filter querying messages, targets, and log levels.
  - Auto-scroll toggle and "Copy to Clipboard" button.

### F. Tray Icon Dynamic Show/Hide & Crawler Status
- **Dynamic Window Toggle**: Left-clicking the system tray icon or selecting the tray menu item toggles between "Show LCU Companion" and "Hide LCU Companion" dynamically based on window visibility.
- **Window Close Interception**: Closing the window hides it and updates the tray menu text to "Show LCU Companion".
- **Unclickable Status Item**: The top item of the tray menu displays live crawler status (e.g., `Crawler: 68.2% (1m 10s)`, `Crawler: Paused`, or `Crawler: Idle`), refreshed every 3 seconds via background Tokio task.

---

## 2. Modified Files

| File | Changes |
| :--- | :--- |
| `src-tauri/src/agent/mod.rs` | Excluded Poro-Snax (`2050`, `2052`) from partner aggregates; increased partner limit to 30; added crawler pause and ETA calculation; added `pause_global_aram_crawler` and `stop_global_aram_crawler` requests. |
| `src-tauri/src/logger.rs` | Added in-memory recent logs buffer, `init_app_handle`, `read_recent_logs`, and Tauri `log_entry` event emission. |
| `src-tauri/src/commands/agent.rs` | Exposed `api_pause_global_aram_crawler`, `api_stop_global_aram_crawler`, and `api_get_logs`. |
| `src-tauri/src/services/history.rs` | Overhauled summoner spell pair scoring; added smart starter item omission (< 5%); enhanced cross-spell regex variable evaluation. |
| `src-tauri/src/lib.rs` | Initialized logger app handle; created dynamic tray menu with crawler status and window toggle; registered new IPC commands. |
| `src/lib/api/agent.ts` | Added `crawler_paused`, `crawler_eta_text` to `GlobalAramConfig`; exported `pauseGlobalAramCrawler`, `stopGlobalAramCrawler`, `getLogs`, and `LogEntry`. |
| `src/lib/components/common/LogsModal.svelte` | Created terminal log modal with level filtering, keyword search, autoscroll, copy, and real-time event streaming. |
| `src/lib/components/TitleBar.svelte` | Added Active Operations pulsing dot, Logs modal button, and Global Database settings & crawler controls dropdown. |
| `src/lib/components/sidebar/ChampionCarousel.svelte` | Added `pinSelectedFirst` prop to pin selected champion to index 0. |
| `src/lib/components/history/MayhemChampSelectView.svelte` | Synchronized champion selection with top-level state; passed `pinSelectedFirst={true}`. |
| `src/lib/components/history/BuildStatisticsView.svelte` | Added sort selectors (Picked, WR, WPA) for associated items and augments; filtered out Poro-Snax; added `GameEntityPopover` rich tooltips; increased display slice to 15. |
| `src/lib/utils/gameDescription.ts` | Added `%i:scaleAP%` inline icon handling and broadened `@Spell...:Var@` cross-spell token parsing. |
| `src/App.svelte` | Rendered crawler progress banner globally across all tabs; wired crawler controls and logger to TitleBar; simplified Mayhem Statistics header. |

---

## 3. Verification & Testing Instructions
- **Champion Pinning**: Open Mayhem > Builds; select any champion. Observe that champion pinned to the very front of the carousel with an amber border. Switch tabs to Builds or Profile and back to confirm persistence.
- **Associated Entities in Statistics**: Open Mayhem > Statistics > Builds. Inspect an item or augment. Confirm no Poro-Snax appear. Toggle sort pills (`Pick`, `WR`, `WPA`) on the Items and Augments columns to verify dynamic sorting. Hover items/augments to verify `GameEntityPopover` rich tooltips appear.
- **Summoner Spells**: View Ashe's Mayhem build; verify Flash / Barrier or Flash / Ghost is recommended instead of Mark.
- **Global Crawler Banner & Controls**: Start crawler; observe the vivid cyan progress bar and ETA row displayed at the top across Builds, Mayhem, and Profile. Open Settings cog in TitleBar and test Pause/Resume and Stop crawler buttons.
- **Logs Modal**: Click the "Logs" button in TitleBar. Check that past logs load, filters (INFO, WARN, ERROR) work, search filters entries, and "Copy" works.
- **Tray Menu**: Left-click tray icon or use menu to toggle window visibility; confirm menu item flips between "Show LCU Companion" and "Hide LCU Companion". Check that the top menu item shows live crawler status.
