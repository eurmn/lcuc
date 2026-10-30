# Walkthrough: Clean Legacy Build Advice & Dedicated Mayhem Build Engine UI

## Overview
Decommissioned legacy heuristic build advice components and refactored the main LCU Companion window (`MayhemChampSelectView.svelte`) to serve as a modular, high-performance UI exclusively presenting data calculated by the modular Mayhem Build Engine (`BuildRecommendationResult`).

---

## 1. Architectural Changes & Decomposition

### Two-Agent Collaboration
1. **Frontend Data & Subcomponent Specialist (`@worker-frontend-data`)**:
   - Designed and built `MayhemBuildEnginePanel.svelte` for build advice presentation (boots, next items, 6-item target path, active combos, engine telemetry).
   - Designed and built `MayhemBuildSimulator.svelte` for 6-slot interactive item simulation and slot targeting.
   - Cleaned TypeScript models in `champ_select.ts`.
2. **Frontend Layout Architect (`@worker-frontend-architect`)**:
   - Extracted `MayhemRosterBar.svelte` for team and bench roster cards during champion select.
   - Completely purged legacy heuristic endpoints (`getMayhemChampionBuild`, `getMayhemTeamBuilds`, `getLiveBuildAdvice`) and removed `LivePurchaseAdvisor.svelte` and `StagePathingTree.svelte`.
   - Refactored `MayhemChampSelectView.svelte` from 2,058 lines down to ~470 lines.

---

## 2. Component Structure

| Component | Responsibility |
| :--- | :--- |
| `MayhemChampSelectView.svelte` | Master orchestrator view: coordinates champion selection, live game state updates, augment simulations, and overlay state synchronization. |
| `MayhemBuildEnginePanel.svelte` | Hero recommendations panel: Recommended Boots, Top 2 Next Items with Lift & Bayesian Win Rate, Greedy 6-Item Target Build, and Engine Latency stats. |
| `MayhemBuildSimulator.svelte` | 6-slot active build simulator: interactive slot targeting, hover-to-clear, total gold calculation, and item popovers. |
| `MayhemRosterBar.svelte` | Allied team & bench roster bar: teammate inspection, local player highlight, swap champion requests. |
| `MayhemCombosCard.svelte` | Combos explorer card: displays item-item and item-augment synergies. |
| `MayhemAugmentPicker.svelte` | 4-slot interactive augment selector. |

---

## 3. Data & State Flow

```
User Action / Live Game State
        │
        ▼
MayhemChampSelectView.svelte
  ├─ Updates (selectedChampionId, userBuildSlots, augmentIds)
  │
  ▼
computeMayhemBuildRecommendation(...)
  │ (Tauri IPC -> lcu-build-engine child process)
  ▼
BuildRecommendationResult
  ├─ Recommended Boots (Bayesian WR, Lift, Sample Size, Reason)
  ├─ Next 2 Items (Bayesian WR, Synergy Lift, Sample Size, Reason)
  ├─ Completed 6-Item Build (Greedy Bitset Synergy Sequence)
  ├─ Active Combos (Item-Item & Item-Augment Pairs)
  └─ Engine Performance Stats (Compute ms, Match Count, Cache Hit)
        │
        ├─► MayhemBuildEnginePanel.svelte (Rendered in Main Window)
        ├─► MayhemBuildSimulator.svelte (Synchronized in Simulator)
        └─► emitOverlayState(...) -> lcu-overlay (Native Overlay HUD Sprites)
```

---

## 4. Verification & Validation
- `bun run check`: **0 errors, 0 warnings** across the TypeScript & Svelte 5 codebase.
- `cargo check --manifest-path src-tauri/Cargo.toml --bins`: Clean build across all targets (`lcu-companion`, `lcu-overlay`, `lcu-companion-agent`, `lcu-build-engine`).
