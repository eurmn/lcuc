# LCU Companion — Feature List

This document provides a comprehensive inventory of the features currently implemented in **LCU Companion**, covering both the native Rust core and the reactive Svelte 5 frontend.

---

## 1. Rust Backend (`src-tauri/`)

- [x] **LCU Auto-Discovery & Authentication** ([`services/lcu_client.rs`](/src-tauri/src/services/lcu_client.rs))
  - Multi-tiered discovery scanning running processes (`sysinfo`) for `LeagueClientUx.exe`.
  - Automatic `lockfile` parsing to extract dynamic port (`--app-port`) and remoting token (`--remoting-auth-token`).
  - Pre-configured HTTPS client (`reqwest`) with custom Riot Games self-signed TLS certificate bypass.

- [x] **Real-Time WebSocket Synchronization** ([`services/lcu_websocket.rs`](/src-tauri/src/services/lcu_websocket.rs))
  - Background asynchronous Tokio WebSocket watcher subscribing to `OnJsonApiEvent`.
  - Automatic detection of active rune page changes and companion slot updates.

- [x] **Summoner Profile & Challenge Rank Service** ([`services/summoner.rs`](/src-tauri/src/services/summoner.rs))
  - Fetches summoner identity, Riot ID, level, and profile icon ID (`/lol-summoner/v1/current-summoner`).
  - Resolves player challenge tier rankings and CDragon challenge token assets (`/lol-challenges/v1/summary-player-data/player/`).

- [x] **Persistent In-Disk CDragon Asset Cache** ([`services/asset_cache.rs`](/src-tauri/src/services/asset_cache.rs))
  - Persistent caching of remote CommunityDragon assets to local disk under `%APPDATA%/com.eurmn.lcu-companion/cache/`.
  - Caches perk icons, profile icons, ranked emblems, and challenge rank tokens.
  - Serves assets to the frontend via local file paths or base64 URIs.

- [x] **SQLite Rune Storage Repository** ([`services/rune_store.rs`](/src-tauri/src/services/rune_store.rs))
  - Embedded zero-configuration SQLite database (`rusqlite`) for custom runesets.
  - Complete repository CRUD: create, list (up to 100 entries), get by ID, and delete.

- [x] **Rune Tree Synthesizer & Client Sync Engine** ([`services/rune_tree.rs`](/src-tauri/src/services/rune_tree.rs), [`services/lcu_perks.rs`](/src-tauri/src/services/lcu_perks.rs))
  - Loads and caches official CDragon style definitions (Precision 8000, Domination 8100, Sorcery 8200, Inspiration 8300, Resolve 8400, and Stat Mods).
  - Selected perk combination synthesizer (`update_selected_perks`) maintaining slot validity.
  - One-click export syncing saved pages to active client slot with prefix `[LCUC]`.
  - One-click import saving live client pages into the local SQLite database.

- [x] **Strongly Typed Tauri 2.0 JSON IPC Endpoints** ([`commands/`](/src-tauri/src/commands/))
  - Profile commands: `api_summoner_profile`, `api_profile_challenge_tier`.
  - Runes overview & CRUD: `api_runes_overview`, `api_runesets`, `api_import_client_page`, `api_export_saved_rune_page`, `api_delete_runeset`.
  - Rune tree interactive commands: `api_rune_tree_current`, `api_rune_tree_client`, `api_rune_tree_saved`, `api_rune_tree_select_perk`, `api_save_rune_tree`.

- [x] **Archived Tools Service** ([`services/friends.rs`](/src-tauri/src/services/friends.rs))
  - Mass friend removal engine with whitelist preservation and audit logging (`purged.txt`).

---

## 2. Svelte Frontend (`src/`)

- [x] **Frameless Title Bar & Controls** ([`TitleBar.svelte`](/src/lib/components/TitleBar.svelte))
  - Custom draggable title bar (`data-tauri-drag-region`).
  - Window minimize, maximize/restore toggle, and close buttons integrated via `@tauri-apps/api/window`.

- [x] **Summoner Profile Card** ([`ProfileCard.svelte`](/src/lib/components/ProfileCard.svelte))
  - Real-time summoner Riot ID, profile icon, and level display.
  - Ranked challenge tier token badge.
  - Graceful "Client Offline" status when the League client is closed.
  - Animated pulse skeleton loading states.

- [x] **Dual-Section Runes Sidebar** ([`RunesSidebar.svelte`](/src/lib/components/sidebar/RunesSidebar.svelte))
  - **Companion Saved Runes** ([`SavedRunesList.svelte`](/src/lib/components/sidebar/SavedRunesList.svelte)):
    - Thematic left icon boxes and style path badges.
    - Active companion badge (`Active` with pulsing purple dot) when equipped in-game.
    - Creation date display (`YYYY-MM-DD`).
    - Smooth hover-only trash delete button.
    - Dynamic count badge and empty state fallback.
  - **League Client Pages** ([`ClientRunesList.svelte`](/src/lib/components/sidebar/ClientRunesList.svelte)):
    - Live client pages with active indicator (`Active` with pulsing green dot).
    - One-click "Import" button with icon and text to save into companion storage.
    - Dynamic count badge and client offline empty state fallback.

- [x] **Interactive Rune Canvas** ([`RuneCanvas.svelte`](/src/lib/components/runes/RuneCanvas.svelte))
  - **Header Bar**:
    - Dot indicators (purple for Saved, blue for Client).
    - Inline editable page name input during edit mode.
    - Contextual action buttons: "Edit", "Export" (push to client), "Import" (save to DB), "Save", and "Cancel".
  - **Two-Column Tree Hierarchy**:
    - Left Column: Primary Path tree with keystone row and 3 tier slots.
    - Right Column: Secondary Path tree and 3 centered rows of Stat Mods.
  - **View Mode**:
    - Displays full perk trees with active runes highlighted with path-specific rings/glows and inactive runes dimmed (`opacity-40`).
    - Slot headers showing slot titles and active rune names in theme accent colors.
  - **Edit Mode**:
    - Clickable perk options with checkmark badges and scale feedback.
    - Instant responsive perk switching via typed IPC commands.

- [x] **Thematic Path Color System** ([`theme.ts`](/src/lib/utils/theme.ts))
  - Path-specific color tokens (borders, glows, rings, badges, text accents):
    - **Precision (8000)**: Amber / Gold
    - **Domination (8100)**: Crimson / Red
    - **Sorcery (8200)**: Brand Purple
    - **Inspiration (8300)**: Turquoise / Cyan
    - **Resolve (8400)**: Emerald / Green

- [x] **Strict TypeScript API Client** ([`src/lib/api/`](/src/lib/api/), [`src/lib/types/`](/src/lib/types/))
  - Shared domain interfaces for all request/response models (`SummonerProfile`, `ChallengeTierProfile`, `RunesOverviewPayload`, `RuneTreePayload`, etc.).
  - Typed `invoke` wrappers with error handling and console logging.

## 3. Unimplemented features

### 3.1 Backend & Frontend

- [ ] Hover over rune icons to display a popover with the description of each rune.
  - Requires Rust to add the rune description to the data sent over to Svelte.
  - Requires new frontend code to implement the popover component.

### 3.2 Svelte Frontend
- [ ] Implement tabs into the title bar
  - Requires new frontend code to add tabs (Runes | Builds | Preferences) to the [`src/lib/components/TitleBar.svelte`] component.
