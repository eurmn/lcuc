# Mayhem Champ Select Situational Item Stat Icons Walkthrough

## Summary
Replaced wide text situational badges (e.g., "Armor Penetration", "Anti-Shield", "Magic Resist", "Anti-Heal") in Mayhem Champ Select with compact CommunityDragon stat icons (`statsicon/`). This applies to both the Stage-Based Build Pathing tree and the Situational Items rack.

## Changes Made

### 1. Situational Tag Mapping Utility (`src/lib/utils/situationalTag.ts`)
- Maps standard backend tags to CommunityDragon stat icon assets:
  - **Armor Penetration** -> `scaleapen.png` (amber accent)
  - **Magic Penetration** -> `scalempen.png` (purple accent)
  - **Magic Resist** -> `scalemr.png` (indigo accent)
  - **Armor / Anti-Burst** -> `scalearmor.png` (yellow accent)
  - **Cleanse / QSS** -> `scaletenacity.png` (emerald accent)
  - **Anti-Shield** -> `scalehealshield.png` (cyan accent)
  - **Anti-Heal** -> `scalehpregen.png` (rose accent)
- Gracefully leaves synergy badges (`Synergy: {:+.0}% WR`) and unmapped tags as text badges.

### 2. Situational Tag Badge Component (`src/lib/components/common/SituationalTagBadge.svelte`)
- Renders an 18px circular badge pinned to the top-right corner of the item container.
- Embeds a 14px CommunityDragon stat icon with category-tailored borders and dark backdrop.
- Includes `title` attribute for native tooltip hover showing full category name.
- Falls back to existing text badge styling when no icon mapping exists.

### 3. Integration in Champ Select Views
- `src/lib/components/history/StagePathingTree.svelte`: Integrated `<SituationalTagBadge tag={it.situational_tag} />`.
- `src/lib/components/history/MayhemChampSelectView.svelte`: Integrated `<SituationalTagBadge tag={sit.situational_tag} />`.

### 4. Unit Tests (`src/lib/utils/situationalTag.test.ts`)
- Runnable test verifying tag mappings, synergy passthrough, and null handling.
