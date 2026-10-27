# Walkthrough: LCU API Connection Status Fix

**Date:** 2026-10-26  
**Scope:** Real-time LCU API Connection Status & Backend Synchronization

---

## 1. Root Cause
- In `MayhemChampSelectView.svelte`, `checkLcuConnection` previously checked `prof && prof.display_name` from `getSummonerProfile()`.
- Modern League accounts use Riot ID (`gameName#tagLine`), leaving `displayName` empty (`""`). This caused `Boolean(prof && prof.display_name)` to constantly evaluate to `false`.
- `refreshSession()` also set `isLcuConnected = false` in its catch handler whenever champ select session polling returned an error, overriding valid active client connections.

---

## 2. Changes
1. **WebSocket Active State Tracking (`src-tauri/src/services/lcu_websocket.rs`)**:
   - Added static `LCU_CONNECTED: AtomicBool`.
   - Exposed `LcuWsWatcher::is_connected()` reflecting real-time WAMP socket connectivity.
2. **LCU Status IPC Command (`src-tauri/src/commands/champ_select.rs`, `src-tauri/src/lib.rs`)**:
   - Added `api_is_lcu_connected` checking `LcuWsWatcher::is_connected()` with fallback credential reachability check against `/lol-gameflow/v1/gameflow-phase`.
3. **Frontend API & View Update (`src/lib/api/champ_select.ts`, `MayhemChampSelectView.svelte`)**:
   - Exported typed `isLcuConnected()` wrapper.
   - Updated `checkLcuConnection` to invoke backend connection status.
   - Guarded `refreshSession` catch block from false offline resets.
   - Re-asserted `isLcuConnected = true` on `lcu:gameflow-phase` and `lcu:champ-select-session` incoming events.
   - Labeled indicator `LCU API: Connected` / `LCU API: Offline`.
