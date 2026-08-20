# Current Project State: LCU Companion (Tauri Desktop Architecture)

This document provides a comprehensive snapshot of the current state of **LCU Companion**, outlining the architecture, tech stack, directory structure, active features, and command integrations.

---

## 1. Architecture Overview

LCU Companion has transitioned from an external FastAPI local server into a **standalone desktop application built with Tauri 2.0 and Rust**.

```
┌────────────────────────────────────────────────────────────────────────┐
│                        Tauri Desktop Application                       │
├───────────────────────────────────┬────────────────────────────────────┤
│       Embedded Frontend           │          Rust Backend              │
│  (index.html + Vite + Tailwind)   │       (src-tauri/src/)             │
│                                   │                                    │
│  • Pure HTML / SSR                │  • Multi-Tiered LCU Discovery:     │
│  • HTMX 2.0                       │    - Lockfile parsing              │
│  • Embedded Tauri HTMX Bridge     │    - sysinfo Process scanning      │
│    (command: prefix routing)      │  • LCU Client (reqwest + TLS)      │
│  • Flowbite Dark Theme            │  • CDragon Rune Cache              │
│                                   │  • SQLite Store (rusqlite)         │
│                                   │  • HTML Partial Renderers          │
│                                   │  • #[tauri::command] Endpoints     │
└───────────────────────────────────┴────────────────────────────────────┘
```

---

## 2. Directory Structure

```
tauri/lcu-companion/
├── index.html                      # Embedded desktop frontend entrypoint
├── package.json                    # Frontend dependencies (htmx, @tauri-apps/api, tailwindcss v4)
├── vite.config.ts                  # Vite build pipeline with @tailwindcss/vite
├── tsconfig.json                   # TypeScript configuration (with DOM.Iterable)
├── src/
│   ├── main.ts                     # HTMX & Tauri bridge initialization
│   └── styles.css                  # Tailwind CSS v4 entrypoint with global cursor rules
└── src-tauri/
    ├── Cargo.toml                  # Rust dependencies (tauri, reqwest, rusqlite, sysinfo, tokio)
    ├── tauri.conf.json             # Tauri 2.0 configuration (1280x720 window, withGlobalTauri)
    └── src/
        ├── main.rs                 # Windows desktop application entrypoint
        ├── lib.rs                  # Tauri builder & command handler registrations
        ├── models/                 # Strongly-typed domain models with `///` IDE docs
        │   ├── profile.rs          # SummonerProfile, ChallengeTierProfile
        │   ├── runes.rs            # RuneTreePayload, RuneSlot, RuneOption, StyleTree
        │   └── database.rs         # RuneSetRecord, RuneSetSummary
        ├── services/               # Core domain & external communication logic
        │   ├── lcu_client.rs       # Multi-tiered lockfile & process scanner, TLS client, tests
        │   ├── summoner.rs         # Summoner profile, regalia borders, challenge tier
        │   ├── rune_tree.rs        # CommunityDragon perk cache, client pages, tree builder
        │   ├── rune_store.rs       # SQLite repository (schema init, save, get, list, delete)
        │   └── friends.rs          # Whitelist check, batch remove unwhitelisted friends, purged.txt logger
        ├── render/                 # Server-side HTML partial renderers for HTMX swaps
        │   ├── profile.rs          # render_profile_card, render_profile_challenge_tier_icon
        │   ├── runes_sidebar.rs    # render_runes_sidebar (dual-section, active badge, selected glow)
        │   ├── rune_tree.rs        # render_rune_tree (keystones h-20, slots, stat mods h-8)
        │   └── shell.rs            # render_shell (Header, Runes tab, Tools tab)
        └── commands/               # #[tauri::command] handlers exposed to HTMX
            ├── profile.rs          # ui_profile_card, ui_profile_challenge_tier, api_summoner_profile
            ├── runes.rs            # ui_runesets, ui_import_client_page, ui_delete_runeset, etc.
            ├── friends.rs          # api_remove_friends
            └── navigation.rs       # ui_shell
```

---

## 3. Active Features & Verified Endpoints

### A. Real-Time Summoner Profile Card
- **Endpoint**: `command:ui_profile_card`
- **Data Verified against live client**:
  - Summoner Riot ID: `Putrefacction#br1`
  - Level: `134`
  - Profile Icon ID: `6004` (CDragon resolved)
  - Regalia Border: Ranked Wings Silver (`ranked`)
- **Challenge Tier**: `command:ui_profile_challenge_tier`

### B. Dual-Section Runes Sidebar
- **Endpoint**: `command:ui_runesets`
- **League Client Pages (Verified live)**:
  - Lists live client pages ("Hecarim Jungle", "Master Yi Jungle", "illaoi", "Porofessor: Swain VS Miss Fortune").
  - Import button (`command:ui_import_client_page`) stores selected page into SQLite.
- **Companion Saved Runes (SQLite)**:
  - Lists and manages persistent custom rune pages with delete button (`command:ui_delete_runeset`).

### C. Center Rune Tree Canvas
- **Endpoints**: `command:ui_rune_tree_current`, `command:ui_rune_tree_client`, `command:ui_rune_tree_saved`.
- Renders full CDragon interactive perk tree with keystones, perk rows, and stat mod rows.
