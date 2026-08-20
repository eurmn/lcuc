# Archived Feature: Tools Navigation & Utilities

## Status: Shelved / Deprecated from UI (August 2026)

### Reason for Archival
The application UI has been streamlined to focus exclusively on the core Rune Tree Companion, real-time client synchronization, and one-click in-game exporting. The top navigation header and the "Tools" tab have been removed from the primary view to maximize screen space for rune management and allow the summoner profile card to float unobtrusively in the top-right corner.

### Existing Functionality
The only functional utility implemented in backend code for the Tools tab was the **Mass Friend Removal Tool**:
- **Backend Service**: [`src-tauri/src/services/friends.rs`](tauri/lcu-companion/src-tauri/src/services/friends.rs)
  - Interacts with `/lol-chat/v1/friends`
  - Allows bulk deletion of League of Legends friend list entries.
- **Tauri Command**: `api_remove_friends` in [`src-tauri/src/commands/friends.rs`](tauri/lcu-companion/src-tauri/src/commands/friends.rs)

### Future Re-introduction Notes
If secondary companion utilities (such as friend cleaner, build recommendations, or match notes) are requested in the future:
1. The backend service and Tauri commands in `services/friends.rs` and `commands/friends.rs` remain preserved in codebase.
2. A drawer/modal or settings trigger can easily mount these tools without cluttering the main rune view.
