# Services Directory (`src-tauri/src/services`)

## Overview
The `services/` directory contains business logic, domain services, external API integrations, and asset caching for the LCU Companion Tauri backend.

## Modules
- `history.rs` (`history.llm.txt`): Match history data fetching from LCU, detail cache persistence, and CommunityDragon item/augment resolution.
- `asset_cache.rs`: Disk and in-memory caching for CommunityDragon images and icons. Converts assets to local files or base64 data URIs.
- `champions.rs`: Champion summary lookup, names, and square icon paths.
- `lcu_client.rs`: HTTP client adapter for communicating with the local League Client Update (LCU) REST API with self-signed SSL support and port/auth discovery.
- `lcu_websocket.rs`: WSS client for subscribing to LCU JSON API events (`OnJsonApiEvent`).
- `rune_service.rs` / `rune_store.rs`: Rune tree validation, SQLite storage of custom runesets, and synchronization with active LCU rune pages.
- `profile.rs`: Summoner profile, ranked queue tier/division emblems, and challenge badges.
