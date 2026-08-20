# Backend Services (`src-tauri/src/services`)

This directory contains the core domain logic, external API integrations, and local storage services for LCU Companion.

---

## Service Modules

### 1. `lcu_client.rs`
- **Purpose**: Discovers the active League of Legends client process (`LeagueClientUx.exe`) using the `sysinfo` crate.
- **Authentication**: Extracts `--app-port` and `--remoting-auth-token` from process arguments to construct a `reqwest::Client` with basic authorization (`riot:<auth_token>`) and self-signed TLS support (`danger_accept_invalid_certs(true)`).

### 2. `summoner.rs`
- **Purpose**: Queries the local LCU endpoints for summoner identity (`/lol-summoner/v1/current-summoner`), regalia borders (`/lol-regalia/v2/current-summoner/regalia`), and challenge tiers (`/lol-challenges/v1/summary-player-data/local-player`).
- **Asset Resolution**: Formats CommunityDragon (CDragon) CDN asset URLs for icons, ranked wings, and challenge tokens.

### 3. `rune_tree.rs`
- **Purpose**: Fetches and caches perk metadata (`perks.json`) and style trees (`perkstyles.json`) from CommunityDragon.
- **Tree Resolution**: Translates raw perk IDs into normalized tree models (Primary Keystones, Tier Slots, Secondary Sub-Style, and Stat Mods) for active client pages, specific client page IDs, and SQLite-persisted runesets.

### 4. `rune_store.rs`
- **Purpose**: Embedded database repository using `rusqlite`.
- **Database File**: Stored in the user's local application data directory under `lcu_companion/lcu_companion.db`.
- **Features**: Automatic table schema initialization, saving client/custom runesets, query by ID with JSON perk deserialization, summary listing, and deletion.
