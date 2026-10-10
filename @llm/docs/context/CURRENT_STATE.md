# Current Project State: LCU Companion (Tauri + Svelte 5 Desktop Architecture)

This document provides a comprehensive snapshot of the current state of **LCU Companion**, outlining the architecture, tech stack, directory structure, active features, and command integrations.

---

## 1. Architecture Overview

LCU Companion is a high-performance desktop application built with **Tauri 2.0 (Rust backend) and Svelte 5 (TypeScript frontend)**.

> **Architecture direction (2026-10-10):** The application is being split into a Tauri UI process and a separately running Rust LCU Companion Agent. The agent will own LCU discovery, websocket/REST synchronization, post-game collection, statistics processing, and the canonical SQLite database. See [`BACKEND_ARCHITECTURE.md`](./BACKEND_ARCHITECTURE.md) and [`@llm/PLAN.md`](../../PLAN.md). The first process/IPC slice is implemented; full cutover is not yet complete.

> **First implementation slice:** A separate agent binary, authenticated loopback IPC, agent SQLite bootstrap/import foundation, Tauri supervision, exhaustive paginated history and shared detail-cache backfill, recent-history reconciliation, an agent websocket listener, and shared application logging now exist. The existing Tauri services remain active until the compatibility cutover is complete.

```
┌────────────────────────────────────────────────────────────────────────┐
│                        Tauri Desktop Application                       │
├───────────────────────────────────┬────────────────────────────────────┤
│       Embedded Frontend           │          Rust Backend              │
│  (Svelte 5 + Vite + TypeScript)   │       (src-tauri/src/)             │
│                                   │                                    │
│  • Svelte 5 Runes ($state, etc.)  │  • Multi-Tiered LCU Discovery:     │
│  • Strict TypeScript Types        │    - Lockfile parsing              │
│  • Tailwind CSS 4.0 Theming       │    - sysinfo Process scanning      │
│  • Typed Tauri IPC Wrappers       │  • LCU Client (reqwest + TLS)      │
│    (invoke("api_*"))              │  • CDragon Rune Cache              │
│  • Thematic League Path Colors    │  • SQLite Store (rusqlite)         │
│                                   │  • Strongly Typed JSON Endpoints   │
└───────────────────────────────────┴────────────────────────────────────┘
```

---

## 2. Directory Structure

```
lcu-companion/
├── index.html                      # Embedded desktop frontend entrypoint (<div id="app">)
├── package.json                    # Frontend dependencies (svelte 5, @tauri-apps/api, tailwindcss v4)
├── svelte.config.js                # Svelte compiler configuration with vitePreprocess
├── vite.config.ts                  # Vite build pipeline with svelte() and @tailwindcss/vite
├── tsconfig.json                   # TypeScript configuration
├── src/
│   ├── main.ts                     # Svelte 5 mount(App, { target: ... }) bootstrap
│   ├── App.svelte                  # Root component (TitleBar, RunesSidebar, RuneCanvas)
│   ├── styles.css                  # Tailwind CSS v4 entrypoint with custom scrollbar
│   └── lib/
│       ├── api/                    # Typed Tauri invoke wrappers (profile.ts, runes.ts, client.ts)
│       ├── components/             # Reactive UI components (TitleBar, ProfileCard, sidebar, runes)
│       ├── types/                  # TypeScript interface definitions (profile.ts, runes.ts)
│       └── utils/                  # Utility helpers & rune theme color palettes (theme.ts)
└── src-tauri/
    ├── Cargo.toml                  # Rust dependencies (tauri, reqwest, rusqlite, sysinfo, tokio)
    ├── tauri.conf.json             # Tauri 2.0 configuration (1280x720 window, withGlobalTauri)
    └── src/
        ├── main.rs                 # Windows desktop application entrypoint
        ├── lib.rs                  # Tauri builder & command handler registrations
        ├── models/                 # Strongly-typed domain models
        │   ├── database.rs         # SQLite entity models
        │   ├── lcu.rs              # LCU API request/response structures
        │   └── runes.rs            # RuneTreePayload, RuneSlot, RuneOption, StyleTree, etc.
        ├── services/               # Core domain & external communication logic
        │   ├── lcu_client.rs       # Multi-tiered lockfile & process scanner, TLS client
        │   ├── summoner.rs         # Summoner profile, regalia borders, challenge tier
        │   ├── rune_tree.rs        # CommunityDragon perk cache, client pages, tree builder
        │   ├── rune_store.rs       # SQLite repository (schema init, save, get, list, delete)
        │   └── friends.rs          # Whitelist check & friend management
        └── commands/               # #[tauri::command] typed JSON IPC endpoints
            ├── profile.rs          # api_summoner_profile, api_profile_challenge_tier
            ├── runes.rs            # api_runes_overview, api_rune_tree_current, api_rune_tree_select_perk, etc.
            └── friends.rs          # api_remove_friends
```

---

## 3. Active Features & Verified Endpoints

### A. Real-Time Summoner Profile Card
- **Endpoints**: `api_summoner_profile`, `api_profile_challenge_tier`
- **Data Model**: `SummonerProfile` and `ChallengeTierProfile`
- Displays Summoner Riot ID, profile icon from CDragon, level, and ranked challenge token.

### B. Dual-Section Runes Sidebar
- **Endpoint**: `api_runes_overview`
- **League Client Pages**: Lists live client pages with active indicator and one-click import into SQLite (`api_import_client_page`).
- **Companion Saved Runes (SQLite)**: Lists saved pages with primary style icons, badges, creation dates, and delete actions (`api_delete_runeset`).

### C. Center Rune Tree Canvas
- **Endpoints**: `api_rune_tree_current`, `api_rune_tree_client`, `api_rune_tree_saved`, `api_rune_tree_select_perk`, `api_save_rune_tree`, `api_export_saved_rune_page`.
- Renders full CDragon interactive perk tree with keystones, standard perk rows, and 3 rows of stat mods.
- Supports both View Mode (highlights active runes and dims unselected options) and Edit Mode (live perk switching and name editing).
