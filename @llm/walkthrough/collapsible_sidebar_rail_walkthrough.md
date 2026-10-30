# Walkthrough: Collapsible Sidebar Rail with Svelte Motion & Rune Builder Refinements

Implemented synchronized layout animations, 130px collapsed rail by default, symmetrical horizontal exponential easing, eliminated FLIP scale distortion, locked keystone options to a single row, enforced unique row selection rules in the secondary rune tree, non-interactive view mode styling, persistent selected rune page in SQLite, uniform 2px borders with mode-specific colors, and generated native executable app icons matching the titlebar logo.

---

## Key Changes

### 1. Native Executable App Icons ([`src-tauri/icons/`](/src-tauri/icons/))
- Ran Tauri icon generation from the application logo ([`src/assets/logo.png`](/src/assets/logo.png)), replacing default boilerplate template assets.
- Generated full multi-resolution icon sets:
  - Windows Executable Icon (`icon.ico` with 16x16, 32x32, 48x48, 64x64, 128x128, 256x256 layers)
  - macOS App Bundle Icon (`icon.icns`)
  - Linux/PNG Icons (`32x32.png`, `64x64.png`, `128x128.png`, `128x128@2x.png`, `icon.png`)
  - Windows Store / Appx Assets (`StoreLogo.png`, `Square44x44Logo.png`, `Square150x150Logo.png`, `Square310x310Logo.png`, etc.)

### 2. Uniform 2px Borders with Mode-Specific Styling ([`RuneCard.svelte`](/src/lib/components/runes/RuneCard.svelte), [`KeystoneCard.svelte`](/src/lib/components/runes/KeystoneCard.svelte), & [`StatShardsCards.svelte`](/src/lib/components/runes/StatShardsCards.svelte))
- All perk cards, keystones, and stat shards maintain a continuous `border-2` (2px) thickness across all states to guarantee **zero row height changes**:
  - **In Editing Mode**:
    - **Selected**: `border-2 ${theme.borderPrimary}` (active theme accent).
    - **Unselected**: `border-2 border-gray-700` (matching the hover border color).
  - **In View Mode**:
    - **Selected**: `border-2 ${theme.borderPrimary}`.
    - **Unselected**: `border-2 border-gray-800/60` (the standard subtle gray border).

### 3. Persistent Selected Rune Page across App Restarts ([`rune_store.rs`](/src-tauri/src/services/rune_store.rs), [`runes.rs`](/src-tauri/src/commands/runes.rs), & [`App.svelte`](/src/App.svelte))
- Added `get_selected_rune_key` and `set_selected_rune_key` to `RuneStore` persisting `selected_rune_key` into the SQLite `app_settings` table.
- Exposed Tauri IPC commands `api_get_selected_rune_key` and `api_set_selected_rune_key` and created typed API wrappers in `src/lib/api/runes.ts`.
- In `App.svelte`:
  - When the user selects, saves, or imports a rune page, the rune key (e.g. `saved:1` or `client:12345`) is automatically persisted to SQLite.
  - On application startup (`onMount`), the persisted rune key is retrieved and opened immediately if valid, falling back to active client or first saved page only if unavailable.

### 4. Non-Interactive View Mode ([`KeystoneCard.svelte`](/src/lib/components/runes/KeystoneCard.svelte), [`RuneCard.svelte`](/src/lib/components/runes/RuneCard.svelte), [`StatShardsCards.svelte`](/src/lib/components/runes/StatShardsCards.svelte), [`PrimaryPathCard.svelte`](/src/lib/components/runes/PrimaryPathCard.svelte), & [`SecondaryPathCard.svelte`](/src/lib/components/runes/SecondaryPathCard.svelte))
- Propagated `isEditing` prop to all perk, keystone, shard, and style path selector cards.
- When `isEditing` is **false** (View Mode):
  - Buttons are disabled (`disabled={!isEditing}`).
  - Cursors are set to `cursor-default` (removing `cursor-pointer`).
  - Hover highlights, background changes, and border glow on hover are disabled.
  - Non-selected perks render with a subtle, non-hoverable `opacity-35 select-none cursor-default` state.

### 5. Secondary Tree Row Constraint ([`RuneCanvas.svelte`](/src/lib/components/runes/RuneCanvas.svelte), [`SecondaryPathCard.svelte`](/src/lib/components/runes/SecondaryPathCard.svelte), & [`SecondaryTreeCards.svelte`](/src/lib/components/runes/SecondaryTreeCards.svelte))
- Updated `handleSelectSecondaryPerk(perkId: number, rowIndex: number)` to inspect which tier rows the existing secondary perks (`selectedPerkIds[4]` and `selectedPerkIds[5]`) occupy:
  - If a newly clicked perk belongs to a row that already has a perk selected, it **replaces the perk in that row**.
  - If the clicked perk is from a 3rd unoccupied row, it replaces the older selected perk via FIFO.
  - Guarantees that users can never select two perks from the same row in the secondary tree.

### 6. Reactive Summoner & Client Lifecycle State Management

#### Backend (Rust Core & LCU WebSocket)
- **Lifecycle & Connection Watcher** ([`lcu_websocket.rs`](/src-tauri/src/services/lcu_websocket.rs)):
  - [`LcuWsWatcher`](/src-tauri/src/services/lcu_websocket.rs) runs continuously in a background Tokio task spawned during Tauri setup via `app.handle().clone()`.
  - Polling loop continuously scans for active League Client processes and lockfile credentials.
  - Employs a `was_connected` guard to ensure connection failure during client startup (e.g. `10061 Connection Refused` before the LCU API port opens) does not trigger false disconnect or change events.
- **Event Bus & Dispatching**:
  - Connects to `wss://127.0.0.1:<port>/` with Basic Auth and unverified TLS, subscribing to WAMP opcode 5 (`OnJsonApiEvent`).
  - Emits typed Tauri desktop events via `tauri::Emitter`:
    - `lcu:connected`: Emitted upon successful WebSocket handshake.
    - `lcu:disconnected`: Emitted when the client connection terminates or resets.
    - `lcu:summoner-changed`: Emitted when receiving events on `/lol-login/v1/session`, `/lol-summoner/v1/current-summoner`, `/lol-regalia/v2/current-summoner/regalia`, or `/lol-challenges/v1/summary-player-data/local-player`.
    - `lcu:runes-changed`: Emitted when `/lol-perks/v1/pages` or `/lol-perks/v1/currentpage` update.
- **Summoner Profile & Regalia Resolution** ([`summoner.rs`](/src-tauri/src/services/summoner.rs)):
  - [`get_current_summoner_profile`](/src-tauri/src/services/summoner.rs) queries `/lol-summoner/v1/current-summoner` and `/lol-regalia/v2/current-summoner/regalia`, resolving Riot ID (`gameName#tagLine`), summoner level, profile icon URL, and ranked wings or theme crests cached on disk via CommunityDragon.
  - [`get_current_challenge_tier_profile`](/src-tauri/src/services/summoner.rs) fetches player challenge level and resolves the cached challenge token asset.

#### Frontend (Svelte 5 & Reactive UI)
- **Profile Card Component** ([`ProfileCard.svelte`](/src/lib/components/ProfileCard.svelte)):
  - Maintains reactive state runes (`$state`):
    - `profile: SummonerProfile | null`
    - `challengeTier: ChallengeTierProfile | null`
    - `loading: boolean` (defaults to `false` to avoid skeleton flicker)
    - `errorMessage: string | null`
  - Listens to `lcu:connected`, `lcu:summoner-changed`, and `lcu:disconnected` via Tauri's `@tauri-apps/api/event`.
  - Executes `refreshProfile(showLoading = false)` for smooth background synchronization when logging in, changing summoners, or updating regalia.
  - Instantly resets to `"Client Offline"` with `profile = null` and `challengeTier = null` upon receiving `lcu:disconnected`.
  - Unregisters all event listeners in `onDestroy`.
- **Global Overview & Sidebar Sync** ([`App.svelte`](/src/App.svelte) & [`RunesSidebar.svelte`](/src/lib/components/sidebar/RunesSidebar.svelte)):
  - `App.svelte` listens to `lcu:connected`, `lcu:runes-changed`, and `lcu:disconnected`, reloading client rune pages (`clientPages`) and clearing active client companion references when disconnected.
  - `RunesSidebar.svelte` exposes `refreshProfile()` and `refreshChampions()` for explicit updates while rendering the profile container seamlessly within the collapsible rail layout.

---

## Verification Results

- Generated native executable icons (`icon.ico`, `icon.png`, `icon.icns`, and Windows Appx sizes) from [`src/assets/logo.png`](/src/assets/logo.png).
- Built executable and bundle will display the exact same icon as the custom titlebar.
