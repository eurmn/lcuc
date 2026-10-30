# Walkthrough — Rune Card Hover Popovers with Rich CDragon Descriptions

Implemented an informative hover popover tooltip feature for rune cards and stat shards across the application using **Flowbite Svelte**, featuring dynamic hover delays (**0.5s** in View Mode vs **1.0s** in Edit Mode), authentic `longDesc` XML/HTML tag parsing, League stat coloration, and inline CDragon stat icons.

---

## 1. Summary of Changes

### A. Backend CDragon Perk Description Enrichment (Rust)
- **Domain Models**:
  - Added `pub long_desc: String` with `#[serde(default)]` to [`RuneOption`](lcu-companion/src-tauri/src/models/runes.rs#L4-L10).
  - Added `pub long_desc: String` with `#[serde(default)]` to [`EnrichedKeystoneOption`](lcu-companion/src-tauri/src/models/rune_builder.rs#L85-L100), [`EnrichedRuneOption`](lcu-companion/src-tauri/src/models/rune_builder.rs#L102-L115), and [`StatShardOption`](lcu-companion/src-tauri/src/models/rune_builder.rs#L143-L155).
- **Rune Tree & Provider Services**:
  - Implemented [`get_perk_details`](lcu-companion/src-tauri/src/services/rune_tree.rs#L133-L155) in `services/rune_tree.rs` to extract `longDesc` (or fallback to `shortDesc`) alongside name and icon URL.
  - Updated `build_style_tree` and `build_stat_mod_slots` in `services/rune_tree.rs` to populate `long_desc`.
  - Updated `services/rune_stats/coachless.rs` across `enrich_rune_items`, `get_keystones`, `get_tree_runes`, and `get_shards` to populate `long_desc` on all enriched options.

---

### B. Frontend Tag Parser Utility (TypeScript)
- Created [`src/lib/utils/runeDescription.ts`](lcu-companion/src/lib/utils/runeDescription.ts) to parse League `longDesc` tags into styled spans with inline stat icons:

| Tag | Color Class | Embedded Icon |
| :--- | :--- | :--- |
| `<scaleAD>` / `<scalead>` | `text-orange-400 font-semibold` | `scalead.png` |
| `<scaleAP>` / `<scaleap>` | `text-blue-400 font-semibold` | `scaleap.png` |
| `<scaleHealth>` / `<scalehealth>` | `text-green-400 font-semibold` | `scalehealth.png` |
| `<scaleMana>` / `<scalemana>` | `text-sky-400 font-semibold` | `scalemana.png` |
| `<scaleLevel>` | `text-amber-200/90 font-semibold` | `scalelevel.png` |
| `<speed>` / `<scaleMS>` | `text-teal-400 font-semibold` | `scalems.png` |
| `<gold>` | `text-yellow-400 font-bold` | `goldcoins.png` |
| `<truedamage>` / `<trueDamage>` | `text-white font-bold` | `enhancedtruedamageicon.png` |
| `<scaleArmor>`, `<scaleMR>`, `<scaleAS>`, `<scaleAH>`, `<scaleAPen>`, `<scaleMPen>`, `<scaleCrit>`, `<scaleTenacity>`, `<scaleAdaptiveForce>`, `<scaleHealShield>`, `<scaleHPRegen>`, `<scaleManaRegen>`, `<scaleCooldown>`, `<scaleRange>`, `<scaleSize>`, `<onhit>`, `<dodgechance>` | Thematic stat colors | Matching CDragon stat icons |
| `<keywordMajor>` | `text-yellow-300 font-bold` | — |
| `<statGood>` | `text-emerald-400 font-semibold` | — |
| `<attention>` | `text-amber-400 font-semibold` | — |
| `<status>` | `text-purple-300 font-semibold` | — |
| `<rules>` | `text-gray-400 italic text-[11px] block my-1` | — |
| `<font color="...">` | Inline `style="color: ..."` | — |
| `<b>`, `<i>` | `font-bold text-white`, `italic text-gray-300` | — |

- Inline icons are rendered with `class="inline-block h-[1em] w-[1em] align-[-0.15em] mr-0.5 object-contain select-none pointer-events-none"`, matching surrounding text size.

---

### C. Flowbite Svelte Popover Integration
- Created [`src/lib/components/runes/RunePopover.svelte`](lcu-companion/src/lib/components/runes/RunePopover.svelte):
  - Uses `Popover` from `flowbite-svelte`.
  - Configures `trigger="hover"`, `triggerDelay={isEditing ? 1000 : 500}`, `placement="top"`, and `arrow={true}`.
  - Formats header with circular rune icon and bold name, divider, and formatted description body.
- Integrated `RunePopover` into:
  - [`KeystoneCard.svelte`](lcu-companion/src/lib/components/runes/KeystoneCard.svelte) with `targetId={`keystone-card-${keystone.id}`}`.
  - [`RuneCard.svelte`](lcu-companion/src/lib/components/runes/RuneCard.svelte) with `targetId={`rune-card-${option.id}`}`.
  - [`StatShardsCards.svelte`](lcu-companion/src/lib/components/runes/StatShardsCards.svelte) with `targetId={`shard-card-${option.id}-${rowIndex}`}`.

---

## 2. File Index

```text
src-tauri/
├── src/
│   ├── models/
│   │   ├── runes.rs                          # Added long_desc to RuneOption
│   │   └── rune_builder.rs                   # Added long_desc to Enriched options & StatShardOption
│   └── services/
│       ├── rune_tree.rs                      # Added get_perk_details and long_desc tree builder mappings
│       └── rune_stats/
│           └── coachless.rs                  # Enriched options with long_desc from CDragon cache

src/
├── lib/
│   ├── types/
│   │   ├── runes.rs                          # TypeScript typing for long_desc in RuneOption
│   │   └── rune_builder.ts                   # TypeScript typing for long_desc in Enriched options
│   ├── utils/
│   │   └── runeDescription.ts                # [NEW] Tag parser & CDragon stat icon definitions
│   └── components/
│       └── runes/
│           ├── RunePopover.svelte            # [NEW] Flowbite Svelte popover component
│           ├── KeystoneCard.svelte           # Connected RunePopover
│           ├── RuneCard.svelte               # Connected RunePopover
│           └── StatShardsCards.svelte        # Connected RunePopover
```

---

## 3. Verification

### Flow & Delay Behavior
1. **View Mode (Default)**: Hovering any rune card or stat shard activates the popover after **0.5 seconds** (500ms).
2. **Edit Mode**: Hovering any rune card or stat shard activates the popover after **1.0 second** (1000ms), giving players ample time to click and customize runes without popup interference.
3. **Stat Icon & Tag Verification**:
   - Attack Damage tags (`<scaleAD>`) display with orange text and `scalead.png`.
   - Ability Power tags (`<scaleAP>`) display with blue text and `scaleap.png`.
   - Health scaling tags (`<scaleHealth>`) display with green text and `scalehealth.png`.
   - Gold tags (`<gold>`) display with gold text and `goldcoins.png`.
   - True damage tags (`<truedamage>`) display with white text and `enhancedtruedamageicon.png`.
   - Color tags (`<font color="#...">`) preserve exact hex colors.
