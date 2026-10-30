# Walkthrough — Tauri Auto-Update with GitHub Releases

## 1. Summary of Changes

Implemented silent background auto-updates for LCU Companion using the official Tauri 2 Updater plugin and GitHub Releases with automated signing via GitHub Actions.

### A. Tauri Updater Plugin & Dependencies
- Added `tauri-plugin-updater = "2"` to `src-tauri/Cargo.toml` for desktop platforms.
- Added `@tauri-apps/plugin-updater` to `package.json`.
- Registered `tauri_plugin_updater::Builder::new().build()` in `src-tauri/src/lib.rs`.
- Granted `updater:default` permission in `src-tauri/capabilities/default.json`.

### B. Minisign Keypair & Repository Secrets
- Generated Ed25519 signing keypair using `tauri signer generate`.
- Configured public key in `src-tauri/tauri.conf.json`.
- Stored `TAURI_SIGNING_PRIVATE_KEY` and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` in GitHub repository secrets on `eurmn/lcuc` via `gh secret set`.

### C. Tauri Configuration
- Updated `src-tauri/tauri.conf.json`:
  - `bundle.createUpdaterArtifacts: true` to generate zipped installers and `.sig` signatures.
  - `plugins.updater.endpoints`: `["https://github.com/eurmn/lcuc/releases/latest/download/latest.json"]`.
  - `plugins.updater.windows.installMode`: `"passive"` (unattended installation with progress bar, automatic app restart).

### D. GitHub Actions Release Workflow
- Created `.github/workflows/release.yml`:
  - Triggers on `push` of `v*` tags or manual `workflow_dispatch`.
  - Builds Windows x86_64 target with `pnpm` and stable Rust toolchain.
  - Uses `tauri-apps/tauri-action@v1` to build, sign artifacts with `TAURI_SIGNING_PRIVATE_KEY`, publish GitHub Release, and upload `latest.json` manifest.

### E. Frontend Silent Update Flow & UI Prompt
- `src/lib/api/updater.ts`:
  - `checkForUpdateSilently()` checks for new release and downloads update package in the background.
  - `applyPendingUpdate()` invokes `update.install({ restartAfterInstall: true })` to apply the update and relaunch.
- `src/App.svelte`:
  - Schedules background update check 5 seconds after mount and every 60 minutes thereafter.
  - Displays a clean prompt banner below the titlebar when an update is downloaded: "LCU Companion vX.Y.Z ready — downloaded in background. Restart application to apply update." with "Restart Now" and "Later" options.
- `src/lib/components/TitleBar.svelte`:
  - Displays an animated green indicator badge (`vX.Y.Z ready`) in the titlebar when an update is downloaded. Clicking re-opens the prompt banner if previously dismissed.

## 2. File Index

| File | Change | Description |
| --- | --- | --- |
| `src-tauri/Cargo.toml` | Modified | Added `tauri-plugin-updater = "2"` dependency |
| `package.json` | Modified | Added `@tauri-apps/plugin-updater` dependency |
| `pnpm-lock.yaml` | Modified | Updated lockfile with updater package |
| `src-tauri/Cargo.lock` | Modified | Updated Cargo lockfile with updater crate |
| `src-tauri/src/lib.rs` | Modified | Registered updater plugin in Tauri builder |
| `src-tauri/capabilities/default.json` | Modified | Added `updater:default` capability permission |
| `src-tauri/tauri.conf.json` | Modified | Configured pubkey, GitHub endpoint, passive install, artifact flag |
| `.github/workflows/release.yml` | Added | Automated GitHub Actions release pipeline with signing |
| `src/lib/api/updater.ts` | Added | Silent update checking, background downloading, and installation API |
| `src/App.svelte` | Modified | Scheduled checks, update state, and restart prompt banner |
| `src/lib/components/TitleBar.svelte` | Modified | Update ready badge and prompt trigger |
| `src-tauri/updater_selfcheck.cjs` | Added | Unit check verifying configuration, endpoints, and workflows |

## 3. Verification

- Ran `node src-tauri/updater_selfcheck.cjs`: verified updater configuration, endpoint format, capabilities, and GitHub Actions pipeline.
- Verified GitHub secrets `TAURI_SIGNING_PRIVATE_KEY` and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` exist on repository `eurmn/lcuc`.
