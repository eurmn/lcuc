# Task Board: task-mayhem-build-optimization

- **Task ID**: `task-mayhem-build-optimization`
- **Task Name**: Mayhem Build Engine Optimization - Frontend & Client-Side Responsiveness
- **Status**: In Progress
- **Created**: 2026-10-31

## Overview
Optimize frontend responsiveness for Mayhem Build Engine:
1. Optimistic slot simulation with debounced IPC in `MayhemBuildSimulator.svelte`
2. Optimistic champ select view with debounced recommendation IPC in `MayhemChampSelectView.svelte`
3. Virtualized rendering of large combo tables in `MayhemCombosCard.svelte`

## Changes Summary
- **MayhemBuildSimulator.svelte**: Add `pending` prop, optimistic local slot state snapshot, subtle "updating" indicator during debounce window
- **MayhemChampSelectView.svelte**: Add 150ms debounce on `computeMayhemBuildRecommendation`, maintain `lastGoodRec` state for flicker-free UI, request-id guard against stale responses
- **MayhemCombosCard.svelte**: Replace page-based pagination with virtualized list (fixed row height, visible window + overscan), keep DOM minimal (~15 visible rows), 60 FPS scrolling

## File Ownership
- Worker: MayhemBuildSimulator.svelte + MayhemChampSelectView.svelte debounce logic
- Worker: MayhemCombosCard.svelte virtualization

## Svelte 5 Patterns Used
- `$state` for local reactive state
- `$derived` for computed values
- `$effect` for side effects and snapshot captures
- Runic `$props()` for typed props
- Derived with `.by()` for expensive computations