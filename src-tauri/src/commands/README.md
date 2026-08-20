# Tauri Commands (`src-tauri/src/commands`)

This directory contains the `#[tauri::command]` handlers that bridge the embedded HTMX frontend with the Rust backend services.

---

## Command Reference

| Command | Arguments | Return Type | HTMX Directive | Purpose |
| :--- | :--- | :--- | :--- | :--- |
| `ui_shell` | `tab: Option<String>` | `String` (HTML) | `hx-get="command:ui_shell"` | Swaps the entire application shell or active navigation tab (`runes` vs `tools`). |
| `ui_profile_card` | None | `String` (HTML) | `hx-get="command:ui_profile_card"` | Returns the summoner profile header card partial with icon and level. |
| `ui_profile_challenge_tier` | None | `String` (HTML) | `hx-get="command:ui_profile_challenge_tier"` | Returns the challenge tier icon partial. |
| `ui_runesets` | `selected_key: Option<String>` | `String` (HTML) | `hx-get="command:ui_runesets"` | Returns the dual-section sidebar listing client and SQLite saved pages. |
| `ui_import_client_page` | `page_id: Option<i64>, pageId: Option<i64>` | `String` (HTML) | `hx-post="command:ui_import_client_page"` | Copies a client page into SQLite and returns the updated sidebar. |
| `ui_delete_runeset` | `runeset_id: Option<i64>, runesetId: Option<i64>` | `String` (HTML) | `hx-post="command:ui_delete_runeset"` | Deletes a page from SQLite and returns the updated sidebar. |
| `ui_rune_tree_current` | None | `String` (HTML) | `hx-get="command:ui_rune_tree_current"` | Returns the active client rune tree canvas partial. |
| `ui_rune_tree_client` | `page_id: Option<i64>, pageId: Option<i64>` | `String` (HTML) | `hx-get="command:ui_rune_tree_client"` | Returns the client rune tree + out-of-band sidebar highlight. |
| `ui_rune_tree_saved` | `runeset_id: Option<i64>, runesetId: Option<i64>` | `String` (HTML) | `hx-get="command:ui_rune_tree_saved"` | Returns the saved rune tree + out-of-band sidebar highlight. |
| `api_summoner_profile` | None | `Result<SummonerProfile, String>` | Frontend invocation | Direct JSON API for summoner profile. |
| `api_runesets` | None | `Result<Vec<RuneSetSummary>, String>` | Frontend invocation | Direct JSON API for saved runesets list. |
