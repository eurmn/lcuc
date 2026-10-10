# LCU Companion — Project Context & Master Guide

Welcome to **LCU Companion**, a modern desktop companion application for **League of Legends (LoL)** built with **Tauri 2.0, Rust, Svelte 5, TypeScript, and Tailwind CSS v4**.

The historical sections below describe the earlier FastAPI/SSR/HTMX architecture. The current frontend is Svelte 5, and the planned next architecture separates the Tauri UI from a standalone Rust LCU Companion Agent. The agent will own LCU synchronization, post-game collection, statistics processing, and the canonical SQLite database. See [`BACKEND_ARCHITECTURE.md`](./BACKEND_ARCHITECTURE.md) and [`@llm/PLAN.md`](../../PLAN.md).

---

## 1. Project Background & Evolution

LCU Companion originated as a Python FastAPI local server that queried the League Client Update (LCU) HTTPS endpoints and rendered HTML templates. In its early evolution, the application was fully migrated to a standalone **Tauri 2.0** desktop app where:
- **Rust Backend (`src-tauri/`)** replaces FastAPI, providing high performance, zero runtime overhead, direct OS process scanning, and embedded SQLite data persistence.
- **Embedded Frontend (`src/`, `index.html`)** uses Vite with Tailwind CSS v4 and HTMX 2.0, talking directly to Tauri backend commands via a custom Tauri-HTMX bridge using `command:<name>` routing.

---

## 2. System Architecture

```
┌────────────────────────────────────────────────────────────────────────┐
│                        Tauri Desktop Application                       │
├───────────────────────────────────┬────────────────────────────────────┤
│       Embedded Frontend           │          Rust Backend              │
│  (index.html + Vite + Tailwind)   │       (src-tauri/src/)             │
│                                   │                                    │
│  • Pure HTML / SSR Layout         │  • Multi-Tiered LCU Discovery:     │
│  • HTMX 2.0 (hx-get, hx-post)     │    - Lockfile parsing              │
│  • Embedded Tauri HTMX Bridge     │    - sysinfo process scan          │
│    (command: prefix routing)      │  • LCU Client (reqwest + TLS)      │
│  • Out-of-Band (OOB) Swaps        │  • CDragon Rune Cache              │
│  • Flowbite Dark Theme            │  • SQLite Store (rusqlite)         │
│                                   │  • HTML Partial Renderers          │
│                                   │  • #[tauri::command] Endpoints     │
└───────────────────────────────────┴────────────────────────────────────┘
```

---

## 3. Directory Layout

```
tauri/lcu-companion/
├── index.html                      # Embedded desktop frontend entrypoint
├── package.json                    # Frontend dependencies (htmx, @tauri-apps/api, tailwindcss v4)
├── vite.config.ts                  # Vite build pipeline with @tailwindcss/vite
├── tsconfig.json                   # TypeScript configuration (with DOM.Iterable)
├── src/
│   ├── main.ts                     # HTMX & Tauri 2.0 bridge initialization
│   └── styles.css                  # Tailwind CSS v4 entrypoint with global cursor rules
├── src-tauri/
│   ├── Cargo.toml                  # Rust dependencies (tauri, reqwest, rusqlite, sysinfo, tokio)
│   ├── tauri.conf.json             # Tauri 2.0 configuration (1280x720 window, withGlobalTauri)
│   └── src/
│       ├── main.rs                 # Desktop application entrypoint
│       ├── lib.rs                  # Tauri builder & command handler registrations
│       ├── models/                 # Strongly-typed domain models with IDE hover docs
│       │   ├── profile.rs          # SummonerProfile, ChallengeTierProfile
│       │   ├── runes.rs            # RuneTreePayload, RuneSlot, RuneOption, StyleTree
│       │   └── database.rs         # RuneSetRecord, RuneSetSummary
│       ├── services/               # Core domain & external communication logic
│       │   ├── lcu_client.rs       # Multi-tiered lockfile & process scanner, TLS client, tests
│       │   ├── summoner.rs         # Summoner profile, regalia borders, challenge tier
│       │   ├── rune_tree.rs        # CommunityDragon perk cache, client pages, tree builder
│       │   ├── rune_store.rs       # SQLite repository (schema init, save, get, list, delete)
│       │   └── friends.rs          # Whitelist check, batch remove unwhitelisted friends, purged.txt logger
│       ├── render/                 # Server-side HTML partial renderers for HTMX swaps
│       │   ├── profile.rs          # render_profile_card, render_profile_challenge_tier_icon
│       │   ├── runes_sidebar.rs    # render_runes_sidebar (dual-section, active badge, selected glow)
│       │   ├── rune_tree.rs        # render_rune_tree (keystones h-20, slots, stat mods h-8)
│       │   └── shell.rs            # render_shell (Header, Runes tab, Tools tab)
│       └── commands/               # #[tauri::command] handlers exposed to HTMX
│           ├── profile.rs          # ui_profile_card, ui_profile_challenge_tier, api_summoner_profile
│           ├── runes.rs            # ui_runesets, ui_import_client_page, ui_delete_runeset, etc.
│           ├── friends.rs          # api_remove_friends
│           └── navigation.rs       # ui_shell
└── llms/docs/                      # AI documentation repository
    ├── context/                    # Architecture, current state, migration history
    ├── lcu/                        # LCU API guides, swagger, websocket, CDragon samples
    ├── db/                         # SQLite database schema and rune modeling
    ├── ui/                         # UI specifications, HTMX reference, Tailwind links
    └── historical/                 # Initial implementation notes, plans, and logs
```

---

## 4. Key Documentation Links

### Context & Architecture
- [`CURRENT_STATE.md`](tauri/lcu-companion/llms/docs/context/CURRENT_STATE.md): Live architecture, components, verified endpoints.
- [`MIGRATION.md`](tauri/lcu-companion/llms/docs/context/MIGRATION.md): Complete chronicle of FastAPI $\rightarrow$ Tauri migration, architectural lessons, and prompt tips.
- [`DOCS_LINKS.md`](tauri/lcu-companion/llms/docs/context/DOCS_LINKS.md): Master catalog of all documentation files.

### LCU Integration & Protocols
- [`LCU.md`](tauri/lcu-companion/llms/docs/lcu/LCU.md): Protocol details, authentication, endpoints, websocket usage.
- [`swagger.json`](tauri/lcu-companion/llms/docs/lcu/swagger.json): Full League Client OpenAPI specification.

### Database & Persistence
- [`DATABASE.md`](tauri/lcu-companion/llms/docs/db/DATABASE.md): SQLite schema definitions and queries.
- [`RUNES.md`](tauri/lcu-companion/llms/docs/db/RUNES.md): Rune page data contracts and modeling.

### UI & Styling
- [`UI.md`](tauri/lcu-companion/llms/docs/ui/UI.md): UI styling principles, components, layout specs.
- [`RUNE_TREE_PLAN.md`](tauri/lcu-companion/llms/docs/ui/RUNE_TREE_PLAN.md): Center canvas rune tree layout specifications.

---

## 5. Development Quickstart

```bash
# 1. Navigate to the project root
cd tauri/lcu-companion

# 2. Install frontend dependencies
pnpm install

# 3. Launch the Tauri desktop companion in development mode
pnpm tauri dev
```
