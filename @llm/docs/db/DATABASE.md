# SQLite Database Architecture & Schema

This document details the database architecture, schema definitions, and repository layer used in LCU Companion.

---

## 1. Overview & Engine

LCU Companion uses **SQLite** as its lightweight, local embedded database engine. Database interactions are completely asynchronous, powered by **`aiosqlite`** to prevent blocking FastAPI's async event loop.

- **Database file location**: `api/data/lcu_companion.db`
- **Repository module**: [`api/services/rune_store.py`](api/services/rune_store.py)
- **Automatic Initialization**: Directories and table schemas are initialized on demand if they do not exist.

---

## 1.1 Planned canonical database ownership

The current SQLite store is opened by Tauri services and primarily contains rune pages, favorite champions, and app settings. The planned architecture introduces a separate LCU Companion Agent with a canonical, versioned SQLite database. The agent will be the single writer for runes, collected games, participants, items, runes, augments, raw response metadata, sync cursors, retry state, and statistics projections.

Migration requirements:

1. Back up the existing Tauri database before changing ownership.
2. Import existing `runesets`, `favorite_champions`, and `app_settings` idempotently.
3. Record schema and import versions plus an import report.
4. Keep Tauri from opening the canonical database after cutover; use the agent API instead.

The agent now has an additive `game_build_facts` table for participant-level champion,
result, item, augment, combat, and economy facts collected with recent games. The
authenticated `build_statistics` IPC request exposes these persisted facts. The
`statistics` IPC request now aggregates these facts in the agent and returns
typed, query-shaped rows for the Statistics page. Svelte must not rebuild
statistics from match-shaped payloads. Full normalized projections and final
Tauri cutover remain planned as specified in
[`@llm/PLAN.md`](../../PLAN.md) and [`../context/BACKEND_ARCHITECTURE.md`](../context/BACKEND_ARCHITECTURE.md).

The agent briefly caches ordered statistics results by scope, kind, filters, and
sort parameters. Pagination is applied to those cached rows, so page changes
remain server-side IPC requests without rebuilding every aggregate.

The agent also maintains a separate `%LOCALAPPDATA%\\lcu-companion\\global-aram.db`
database for the opt-in global ARAM: Mayhem dataset. It contains the configured
game limit, discovered participant PUUIDs, deduplicated ARAM games, and participant
build facts. The collector starts from the current account, follows participant
PUUIDs breadth-first, uses bounded concurrent detail requests, and stops when the
configured limit (default 1000 games) is reached.

Standalone-agent SQLite connections use WAL mode, `NORMAL` synchronous operation,
in-memory temporary storage, and a five-second busy timeout. Connections are
opened per operation and are not held across network requests or JSON parsing,
which keeps collector writes from unnecessarily blocking UI reads.

## 2. Table Schemas

### `runesets` Table

The `runesets` table stores app-managed saved rune pages, whether imported manually from the League client, captured automatically during champion select, or created within the application.

```sql
CREATE TABLE IF NOT EXISTS runesets (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    summoner_puuid TEXT,
    champion_id INTEGER,
    game_id INTEGER,
    rune_page_id INTEGER,
    rune_page_name TEXT,
    primary_style_id INTEGER,
    sub_style_id INTEGER,
    selected_perk_ids_json TEXT NOT NULL,
    source TEXT NOT NULL,
    role INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS favorite_champions (
    champion_id INTEGER PRIMARY KEY,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS app_settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
```

### `favorite_champions` Table
Stores user-favorited champions to pin them at the front of champion selectors and searches.

| Column | Type | Nullable | Description |
| :--- | :--- | :--- | :--- |
| `champion_id` | `INTEGER` | `NO` | Champion ID marked as favorite (Primary Key). |
| `created_at` | `TEXT` | `NO` | Timestamp when the champion was favorited. |

### `app_settings` Table
Key-value configuration store for persisting user preferences (such as `selected_champion_id`).

| Column | Type | Nullable | Description |
| :--- | :--- | :--- | :--- |
| `key` | `TEXT` | `NO` | Setting key name (Primary Key). |
| `value` | `TEXT` | `NO` | Setting value payload. |
| `updated_at` | `TEXT` | `NO` | Last update timestamp. |

---

| Column | Type | Nullable | Description |
| :--- | :--- | :--- | :--- |
| `id` | `INTEGER` | `NO` | Auto-incrementing primary key. |
| `summoner_puuid` | `TEXT` | `YES` | Riot PUUID of the player account owning the rune page. |
| `champion_id` | `INTEGER` | `YES` | Champion ID if the rune page was captured or assigned to a specific champion. |
| `game_id` | `INTEGER` | `YES` | Match/Game ID if the page was captured from game history or champion select. |
| `rune_page_id` | `INTEGER` | `YES` | Original LCU rune page ID from the League client (for imported pages). |
| `rune_page_name` | `TEXT` | `YES` | Name of the rune page (e.g. `"Conqueror Bruiser"`). |
| `primary_style_id` | `INTEGER` | `YES` | Primary Style ID (e.g. `8000` Precision, `8100` Domination, `8200` Sorcery, `8300` Inspiration, `8400` Resolve). |
| `sub_style_id` | `INTEGER` | `YES` | Secondary Style ID. |
| `selected_perk_ids_json` | `TEXT` | `NO` | JSON array of 9 selected perk IDs: `[keystone, t1, t2, t3, s1, s2, stat1, stat2, stat3]`. |
| `source` | `TEXT` | `NO` | Origin tag: `"league_client"`, `"champ_select"`, `"custom"`. |
| `created_at` | `TEXT` | `NO` | Creation timestamp (`CURRENT_TIMESTAMP` in UTC format). |

---

## 3. Repository Layer (`RuneSetRepository`)

All queries and database mutations are encapsulated in [`RuneSetRepository`](api/services/rune_store.py#L38):

```python
repo = RuneSetRepository()
```

### Methods

- **`initialize_schema() -> None`**  
  Creates the parent directory (`api/data/`) and executes the `CREATE TABLE IF NOT EXISTS` statement.

- **`save_runeset(...) -> int`**  
  Serializes `selected_perk_ids` into a JSON string and inserts a new record into `runesets`. Returns the newly created row ID (`int`).

- **`get_runeset(runeset_id: int) -> Optional[RuneSetRecord]`**  
  Fetches a single record by ID, parses `selected_perk_ids_json` into a Python `List[int]`, and returns a typed dictionary.

- **`list_runesets(limit: int = 50) -> List[RuneSetSummary]`**  
  Retrieves saved rune page summaries ordered by `id DESC` (most recent first) up to the specified limit.

- **`delete_runeset(runeset_id: int) -> bool`**  
  Deletes a saved rune page by `id`. Returns `True` if a row was deleted, `False` otherwise.

---

## 4. Concurrency & Performance

- **Async Context Management**: All operations use `async with aiosqlite.connect(self.db_path) as db:`, opening and closing connections per transaction.
- **JSON Serialization**: Storing the 9 selected perks as JSON array (`[8010, 9111, 9104, 8014, 8226, 8236, 5008, 5008, 5002]`) provides schema flexibility while avoiding unnecessary relational overhead for fixed-size perk lists.
