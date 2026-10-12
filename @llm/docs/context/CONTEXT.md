# LCU Companion — Project Context & Master Guide

Welcome to **LCU Companion**, a desktop companion application for **League of Legends** built with **Tauri 2.0 (Rust backend), Svelte 5 (TypeScript frontend), Tailwind CSS v4, and a Standalone Rust Agent (`lcu-companion-agent`)**.

---

## 1. System Architecture

```text
┌────────────────────────────────────────────────────────────────────────┐
│                        Tauri Desktop Application                       │
├───────────────────────────────────┬────────────────────────────────────┤
│       Embedded Frontend           │          Tauri Shell & IPC         │
│  (Svelte 5 + Vite + TypeScript)   │       (src-tauri/src/)             │
│                                   │                                    │
│  • Svelte 5 Runes ($state, etc.)  │  • Supervised agent lifecycle      │
│  • Flowbite-Svelte UI Patterns    │  • Authenticated loopback IPC      │
│  • Tailwind CSS 4.0 Theming       │  • Legacy direct services fallback │
│  • Progress Banner for Background │  • System tray & window management │
│    Jobs (crawlers, projections)   │                                    │
└───────────────────────────────────┴────────────────────────────────────┘
                                   │ Authenticated Loopback IPC
                                   ▼
┌────────────────────────────────────────────────────────────────────────┐
│              Standalone Agent (`lcu-companion-agent`)                  │
├────────────────────────────────────────────────────────────────────────┤
│  • LCU Auto-Discovery (Process / Lockfile scanner & TLS)               │
│  • WebSocket Listener (WSS `OnJsonApiEvent`) & Event Reconnection     │
│  • Data Collection: Personal Match Reconciler & Background Backfill    │
│  • Global ARAM Crawler: Multi-node BFS player crawl & match ingestion  │
│  • Precomputed Projections: SQLite `statistics_projections` & caches   │
│  • Compare Mode Aggregation: Personal vs. Global WR, WPA, KDA, items  │
│  • SQLite Storage Owner (`agent.db` and `global-aram.db`)              │
└────────────────────────────────────────────────────────────────────────┘
```

---

## 2. Directory Layout

```text
lcu-companion/
├── index.html                      # Embedded desktop frontend entrypoint (<div id="app">)
├── package.json                    # Frontend dependencies (Svelte 5, Tailwind v4, Flowbite-Svelte)
├── vite.config.ts                  # Vite build pipeline with svelte() and @tailwindcss/vite
├── tsconfig.json                   # TypeScript configuration
├── src/
│   ├── main.ts                     # Svelte 5 application entrypoint
│   ├── App.svelte                  # Root component (TitleBar, sidebar, canvas, progress banner)
│   ├── styles.css                  # Tailwind CSS v4 entrypoint
│   └── lib/
│       ├── api/                    # Typed IPC wrappers (agent.ts, profile.ts, runes.ts, client.ts)
│       ├── components/             # UI components
│       │   ├── history/            # Match history & Compare statistics (Builds, Champions)
│       │   ├── runes/              # Interactive Rune Canvas components
│       │   └── sidebar/            # Saved runes & client pages lists
│       ├── types/                  # TypeScript interface definitions
│       └── utils/                  # Theme colors, helpers, shared logger
└── src-tauri/
    ├── Cargo.toml                  # Rust dependencies
    ├── tauri.conf.json             # Tauri 2.0 configuration
    └── src/
        ├── bin/
        │   └── lcu-companion-agent.rs # Standalone agent executable entrypoint
        ├── agent/                  # Standalone agent core (IPC, DB, crawler, stats, projections)
        ├── agent_supervisor.rs     # Tauri child process manager for agent
        ├── commands/               # Tauri IPC command facades
        ├── services/               # Legacy/direct services (runes, profile, cache)
        ├── lib.rs                  # Tauri builder & setup
        └── main.rs                 # Windows native entrypoint
```

---

## 3. Database Ownership & Storage

The standalone agent is the sole writer for application data located in `%LOCALAPPDATA%\lcu-companion\`:

- `agent.db`: Personal account identities, recent games, `game_build_facts`, and precomputed projections.
- `global-aram.db`: Discovered crawler accounts (`discovered_accounts`), global ARAM matches, `aram_build_facts`, and global projections.
- Connections enforce WAL mode, a 5-second busy timeout, and operation-scoped lifecycles.
- Synchronous SQLite work in async code executes inside `tokio::task::spawn_blocking`.

---

## 4. Key Documentation Index

- [`CURRENT_STATE.md`](./CURRENT_STATE.md): Live snapshot of features, endpoints, and crawler status.
- [`SESSION_HANDOFF_2026-10-11.md`](./SESSION_HANDOFF_2026-10-11.md): Handoff details for agent stabilization.
- [`BACKEND_ARCHITECTURE.md`](./BACKEND_ARCHITECTURE.md): Architecture split and target direction.
- [`../db/DATABASE.md`](../db/DATABASE.md): SQLite schema definitions and queries.
- [`../lcu/LCU.md`](../lcu/LCU.md): LCU protocol, auth headers, and WebSocket specs.

---

## 5. Development Quickstart

```powershell
# 1. Build standalone agent binary
cargo build --manifest-path src-tauri\Cargo.toml --bin lcu-companion-agent

# 2. Launch frontend and Tauri in development mode
pnpm tauri dev
```
