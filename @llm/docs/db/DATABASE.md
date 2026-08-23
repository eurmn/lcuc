# SQLite Database Architecture & Schema

This document details the database architecture, schema definitions, and repository layer used in LCU Companion.

---

## 1. Overview & Engine

LCU Companion uses **SQLite** as its lightweight, local embedded database engine. Database interactions are completely asynchronous, powered by **`aiosqlite`** to prevent blocking FastAPI's async event loop.

- **Database file location**: `api/data/lcu_companion.db`
- **Repository module**: [`api/services/rune_store.py`](api/services/rune_store.py)
- **Automatic Initialization**: Directories and table schemas are initialized on demand if they do not exist.

---

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
