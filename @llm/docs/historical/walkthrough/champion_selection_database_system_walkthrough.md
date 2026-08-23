# Walkthrough: Unified Champion Selection State, Favorite System & SQLite Architecture

We have overhauled the champion selection state, interactive champion switching, and champion favoriting system across the full stack—from SQLite storage and Tauri IPC endpoints to reactive Svelte 5 frontend components using Flowbite and Heroicons.

---

## 1. Key Features & Implementation

### A. Persistent Favorite Champions (`favorite_champions` Table)
- **`src-tauri/src/services/rune_store.rs`**:
  - Initialized `favorite_champions (champion_id INTEGER PRIMARY KEY, created_at TEXT)`.
  - Added `get_favorite_champion_ids()` and `toggle_favorite_champion(champion_id: i64)`.
- **`src-tauri/src/services/champions.rs`**:
  - `get_all_champions` and `get_saved_champion_roster` populate `is_favorite: bool`.
- **`src-tauri/src/commands/champions.rs`**:
  - Added `api_toggle_favorite_champion(champion_id: i64) -> Result<bool, String>`.

### B. Interactive Champion Switching & Flowbite Modal
- **`src/lib/components/ChampionSelectModal.svelte`**:
  - Flowbite-styled modal for selecting or re-assigning champion associations.
  - Search input with quick-clear and "All Champions (Universal)" option.
  - Lists champions sorted: **Favorites first**, then **champions with saved runes**, then alphabetical.
- **`src/lib/components/sidebar/SavedRunesList.svelte`**:
  - Champion avatar on each saved rune page row is clickable to open the Flowbite Champion Select Modal and re-assign that page's champion directly in SQLite.
- **`src/lib/components/runes/RuneCanvas.svelte`**:
  - Header champion avatar in Edit Mode features a hover edit overlay (`icon-[heroicons--pencil]` and "EDIT" badge). Clicking opens the Champion Select Modal.

### C. Favorite Toggle Button & Roster Priority
- **`src/lib/components/sidebar/SelectedChampionSlot.svelte`**:
  - Displays a solid bookmark button (`icon-[heroicons--bookmark-20-solid]`) at 1.75rem (`h-7 w-7`) overlaying the top-left of the selected champion avatar.
  - Filled purple (`text-purple-400`) when favorited, and lightest slate (`text-slate-100`) with soft drop shadow when unselected, with an opacity hover effect only (no scale/color change).
- **`src/lib/components/sidebar/RunesSidebar.svelte`**:
  - **Selected Slot**: The dedicated `SelectedChampionSlot` is pinned on the far left of the divider. When no champion is selected, it displays the "All Champions" icon.
  - **Ordering**: Favorite champions appear first in the carousel. The "All Champions" option in the carousel only displays when a champion is actively selected, allowing quick deselection.
  - **Theming**: Favorite champions inside the champion list have a clean **2px yellow border (`border-2 border-yellow-400`)**.

### D. Icon System Migration to Heroicons
- Replaced raw inline `<svg>` elements across all components with `@iconify-json/heroicons` icon classes (`icon-[heroicons--...]`).
- Documented in [GEMINI.md](lcu-companion/GEMINI.md#L190-L200) and [DATABASE.md](lcu-companion/@llm/docs/db/DATABASE.md#L38-L50).

### E. Decoupled Rune Champion & Active Tree Flowbite Indicator
- **State Decoupling**: Changing the sidebar champion filter (`selectedChampion`) filters the saved rune page list without modifying the currently loaded rune tree or its assigned champion.
- **Edit Mode Synchronization**: In `RuneCanvas`, editing the rune tree's champion updates the rune tree and automatically updates the sidebar filter champion.
- **Active Tree Warning Indicator Circle**: When the open rune page belongs to a champion different from the currently selected champion, that champion item in the list displays a small blue indicator circle (`bg-blue-500 rounded-full`) at `top-[2px] right-[2px]` with a dark blue shadow (`shadow-[0_1px_3px_rgba(29,78,216,0.85)]`) and no border. When the open rune belongs to the selected champion, no indicator is shown.

### F. Selected Champion Persistence in SQLite
- **`app_settings` Table**: Added `app_settings` key-value table (`key TEXT PRIMARY KEY, value TEXT NOT NULL, updated_at TEXT NOT NULL`) in SQLite.
- **IPC Endpoints**: `api_get_selected_champion_id` and `api_set_selected_champion_id` retrieve and store the `selected_champion_id` preference.
- **Auto-Restoration**: On app startup/refresh, `App.svelte` restores the last selected champion from SQLite (or defaults to All Champions if none was ever selected).

### G. Active Rune Page Visibility Across Champion Filters
- **Cross-Champion Active Rune Visibility**: In `RunesSidebar`, if the currently active/selected rune page belongs to a different champion than the active filter, it is dynamically prepended at the top of the Companion Saved Runes list (before the current champion's saved runes).

---

## 2. Verification

- `pnpm svelte-check --threshold error` passed with 0 errors.
- Schema initialized cleanly with zero migration conflicts.
- Flowbite modal and Heroicon classes verified.
