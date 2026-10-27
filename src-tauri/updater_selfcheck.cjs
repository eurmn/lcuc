const assert = require("node:assert");
const fs = require("node:fs");

// Check tauri.conf.json
const tauriConfig = JSON.parse(fs.readFileSync("src-tauri/tauri.conf.json", "utf8"));
assert.strictEqual(tauriConfig.bundle?.createUpdaterArtifacts, true, "createUpdaterArtifacts must be true");
assert.ok(tauriConfig.plugins?.updater?.pubkey, "pubkey must exist");
assert.ok(
  tauriConfig.plugins.updater.endpoints.includes(
    "https://github.com/eurmn/lcuc/releases/latest/download/latest.json"
  ),
  "endpoint must point to eurmn/lcuc releases latest.json"
);
assert.strictEqual(
  tauriConfig.plugins.updater.windows?.installMode,
  "passive",
  "windows installMode must be passive"
);

// Check capabilities
const caps = JSON.parse(fs.readFileSync("src-tauri/capabilities/default.json", "utf8"));
assert.ok(caps.permissions.includes("updater:default"), "updater:default permission required");

// Check release workflow
const workflow = fs.readFileSync(".github/workflows/release.yml", "utf8");
assert.ok(workflow.includes("tauri-apps/tauri-action@v1"), "tauri-action@v1 required");
assert.ok(workflow.includes("TAURI_SIGNING_PRIVATE_KEY"), "TAURI_SIGNING_PRIVATE_KEY required");
assert.ok(workflow.includes("tagName: v__VERSION__"), "tagName v__VERSION__ required");

console.log("Updater selfcheck passed.");
