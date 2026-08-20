# LCU Integration Guide

This document explains how the app talks to the League Client Update (LCU) API today, why the current architecture exists, and where the next agent can extend it safely.

## 1) Core LCU model used by this app

LCU is a **local HTTPS API** exposed by the running Riot client, on a dynamic port, with short-lived credentials.  
This app does not depend on `lcu_driver` anymore; it has its own typed LCU client in `api/services/lcu_api.py`.

### Authentication flow

1. Find `LeagueClientUx(.exe)` in running processes.
2. Read command-line args:
   - `--app-port=<port>`
   - `--remoting-auth-token=<token>`
3. Build base URL:
   - `https://127.0.0.1:<port>`
4. Use HTTP Basic auth:
   - username: `riot`
   - password: `<token>`
5. Disable HTTPS certificate verification (LCU uses self-signed certs).

### Readiness flow

After credentials are found, the app waits until API readiness:

- Poll `GET /riotclient/region-locale`
- Continue when it responds (current behavior accepts any non-5xx response)

This allows backend startup before client startup.

## 2) Current module map

- `api/services/lcu_api.py`
  - process discovery + credential parsing
  - API readiness polling
  - typed HTTP wrappers:
    - `request_json(...)`
    - `request_status(...)`

- `api/services/lcu.py`
  - business-level LCU operations:
    - `get_current_summoner_profile()`
    - `remove_friends()`
  - payload shaping + type checks
  - profile border selection logic

- `api/services/cdragon.py`
  - CommunityDragon asset fetch/cache:
    - profile icon
    - ranked wings border
    - themed border
  - local cache root:
    - `api/static/assets/cdragon`

- `api/main.py`
  - API endpoints exposed to frontend:
    - `GET /api/summoner-profile`
    - `POST /api/remove-friends`
    - `GET /api/health`

## 3) Current profile pipeline

Frontend calls `GET /api/summoner-profile`. Backend does:

1. `GET /lol-summoner/v1/current-summoner`
   - resolves: displayName, gameName, tagLine, summonerLevel, profileIconId
2. `GET /lol-regalia/v2/current-summoner/regalia`
   - resolves: crestType, highestRankedEntry.tier, selectedPrestigeCrest
3. Asset resolution:
   - icon: CDragon profile-icons by `profileIconId`
   - ranked border: `ranked-emblem/wings/<tier>.png`
   - non-ranked border: `uikit/themed-borders/theme-<id>-border.png`
4. Return normalized profile JSON for UI rendering.

## 4) Current friend-removal pipeline

`POST /api/remove-friends` does:

1. Confirm LCU connectivity
2. `GET /lol-chat/v1/friends`
3. Compare each friend with local whitelist (`whitelist.txt`)
4. `DELETE /lol-chat/v1/friends/{id}` for non-whitelisted entries
5. Append removals to `purged.txt`

## 5) Websocket usage path (recommended next)

For real-time UI updates (icon/level/border), use LCU websocket:

- Connect: `wss://127.0.0.1:<port>/` with same Basic auth
- Disable TLS verification
- Subscribe:
  - send `[5, "OnJsonApiEvent"]`
- Watch for URI events:
  - `/lol-summoner/v1/current-summoner`
  - `/lol-regalia/v2/current-summoner/regalia`
- On event, refresh current profile from HTTP endpoints and push to UI

This keeps data fresh without aggressive polling.

## 6) Endpoints used today

- `GET /riotclient/region-locale` (readiness)
- `GET /lol-summoner/v1/current-summoner` (identity/profile)
- `GET /lol-regalia/v2/current-summoner/regalia` (border metadata)
- `GET /lol-chat/v1/friends` (friend list)
- `DELETE /lol-chat/v1/friends/{id}` (friend removal)

## 7) Candidate endpoints for future features

These are high-value, practical next calls for this project:

### Profile and identity
- `GET /lol-chat/v1/me`  
  Useful for chat-presence-facing identity fields and consistency checks.

### Ranked/regalia
- `GET /lol-ranked/v1/current-ranked-stats`  
  Rich ranked state, queue breakdown, highest ranked entry.

### Runes and loadouts
- `GET /lol-perks/v1/pages`  
  Read current rune pages for the Runes tab.
- `POST /lol-perks/v1/pages` / `PUT /lol-perks/v1/pages/{id}` / `DELETE ...`  
  Create/edit/delete rune pages.

### Champ Select integration
- `GET /lol-champ-select/v1/session`  
  Detect current pick phase and selected champion.
- `GET /lol-champ-select/v1/current-champion`  
  Quick champion context for automated rune suggestions.

### Summoner spell + profile signals
- `GET /lol-summoner/v1/current-summoner/summoner-profile`  
  Additional profile-level metadata when needed.

## 8) Data contracts you can extend safely

Current profile payload from `/api/summoner-profile` includes:

- `riot_id`
- `display_name`
- `summoner_level`
- `profile_icon_id`
- `profile_icon_asset_url`
- `profile_border_asset_url`
- `profile_border_kind`

Recommended extension strategy:

1. Extend `SummonerProfile` TypedDict first.
2. Add strict parser helpers (`_as_int`, `_as_string`, etc.).
3. Keep optional fields nullable (`Optional[...]`) to avoid brittle UI behavior.

## 9) Operational notes

- LCU can disappear at any moment (client close/restart). Reconnect logic is essential.
- Port/token rotate on client restart. Never cache credentials long-term.
- Always handle non-200 responses explicitly; LCU errors are common during transitions.
- Asset cache should be considered best-effort: if CDragon download fails, return profile without border/icon URLs.

## 10) Quick extension checklist for the next agent

1. Add endpoint contract in `api/main.py`.
2. Implement LCU call in `api/services/lcu.py` using `LcuApiClient`.
3. Parse with strict type guards.
4. If visual asset: cache through `api/services/cdragon.py`.
5. Update frontend loader/render path in `templates/index.html` or component templates.
6. Add short doc update in `README.md` and deeper details in this file.
