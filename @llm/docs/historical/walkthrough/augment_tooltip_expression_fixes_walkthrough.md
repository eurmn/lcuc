# Walkthrough: Fix Glitched Augment Descriptions

## Problem & Root Causes
1. **Forged by the Master**:
   - *Previous state*: `"Your Item and Augment damage are increased by 0.44999998807907104. Bonus Damage: @f1.0@"`
   - *Cause*: Riot's `Calc_Damage_Amp` calculation has `mDisplayAsPercent: true` with a base float of `0.44999998807907104`. The calculation was rendered without percent conversion, and trailing runtime match trackers wrapped in `<rules>` were not scrubbed.
2. **Combusting Interest (BloodMoneyBurn)**:
   - *Previous state*: `"Burn and damage-over-time on champions generate 1 - 3 + 1% of damage dealt as Gold. Gold Generated:"`
   - *Cause*: `0.0075 * 100` (`0.75%`) was rounded to an integer (`1%`), and stripping the tracker `@f1@` left the label `Gold Generated:` hanging at the end.
3. **Firebrand**:
   - *Previous state*: `"Your Attacks also Burn enemies, dealing @PercentMaxHPDamagePer5Seconds*20@% max Health magic damage damage per second over 5 seconds. This Burn stacks infinitely."`
   - *Cause*: Parameter substitution only checked `(\*100)?`, missing expressions like `*20` (`0.02 * 20 = 0.4%`). Also, raw Riot string contained duplicate `magic damage damage`.

---

## Changes Made

### 1. Augment Data Generator
- In [`scratch/generate_augment_data.js`](scripts/scratch) & [`src-tauri/src/data/augment_tooltips.json`](src-tauri/src/data/augment_tooltips.json):
  - **`mDisplayAsPercent` Support**: Multiplies `GameCalculation` formulas by 100, cleans float noise, and appends `%` (preventing double `%%` if followed by `%` in the template).
  - **Comprehensive Math Expression Parser**: Replaces `@([a-zA-Z0-9_]+)(?:([*+/-])(-?[0-9]+(?:\.[0-9]+)?))?@(%?)`, accurately computing formulas such as `*20`, `*1000`, `*-100`, etc.
  - **Precision Cleaning**: Strips float32 noise (e.g. `0.07999999821186066` -> `0.08`, `0.0075 * 100` -> `0.75%`).
  - **Tracker Stripping**: Removes `<rules>` tracker containers, trailing tracker keywords (e.g. `Gold Generated:`, `Bonus Damage:`, `Damage Dealt:`, `Health Drained:`), and standalone `@f[0-9.]@` variables.
  - **Word Deduplication**: Collapses `magic damage damage` -> `magic damage`.

### 2. Frontend Parser Defense-in-Depth
- In [`src/lib/utils/gameDescription.ts`](src/lib/utils/gameDescription.ts):
  - Added matching tracker keyword cleanup and hanging label trimming.
  - Added raw float32 noise sanitization (`\b([0-9]+\.[0-9]{3,})\b`).
  - Extended parameter fallback regex to handle arithmetic expressions and float numbers (`@([a-zA-Z0-9_]+)(?:[*+/-]-?[0-9.]*)?@(%?)`).

### 3. Backend Cache Version Bump
- In [`src-tauri/src/services/history.rs`](src-tauri/src/services/history.rs#L119):
  - Bumped cache filename to `augments_v8.json` to force reload of the newly sanitized dataset.
