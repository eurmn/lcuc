<script lang="ts">
  import { MotionSpan, type MotionTransition } from "@humanspeak/svelte-motion";
  import SavedRuneItem from "./SavedRuneItem.svelte";
  import type { RuneSetSummary } from "../../types/runes";
  import type { ChampionSummary } from "../../types/champions";

  let {
    items = [],
    selectedKey = "",
    activeCompanionName = null,
    allChampions = [],
    isOpen = true,
    onSelect,
    onDelete,
    onChangeChampion,
  }: {
    items: RuneSetSummary[];
    selectedKey: string;
    activeCompanionName: string | null;
    allChampions?: ChampionSummary[];
    isOpen?: boolean;
    onSelect: (runesetId: number) => void;
    onDelete: (runesetId: number) => void;
    onChangeChampion?: (runesetId: number, currentChampionId: number) => void;
  } = $props();

  let layoutTransition: MotionTransition = $derived({
    x: { duration: 0.35, ease: [0.16, 1, 0.3, 1] },
    width: { duration: 0.35, ease: [0.16, 1, 0.3, 1] },
    y: { duration: 0.35, ease: "linear" },
    height: { duration: 0.35, ease: "linear" },
    opacity: { duration: 0.35, ease: "linear" },
  });
</script>

<div class="w-full min-w-0">
  <!-- Header: smoothly morphs between "Companion Saved Runes" and "SAVED" as width changes -->
  <div class="mb-2.5 flex items-center justify-between px-1 w-full min-w-0 overflow-hidden">
    <div class="flex items-center gap-2 min-w-0 overflow-hidden">
      <span class="flex h-2 w-2 shrink-0 rounded-full bg-purple-500 shadow-sm shadow-purple-500"></span>
      <MotionSpan layout="position" transition={layoutTransition} class="text-xs font-bold uppercase tracking-wider text-gray-300 whitespace-nowrap overflow-hidden">
        {isOpen ? "Companion Saved Runes" : "SAVED"}
      </MotionSpan>
    </div>
    <span class="shrink-0 rounded-full bg-gray-800 px-2 py-0.5 text-[11px] font-semibold text-gray-400 border border-gray-700/60">
      {items.length}
    </span>
  </div>

  {#if items.length === 0}
    <div class="flex flex-col items-center justify-center rounded-2xl border border-dashed border-gray-800 bg-gray-900/30 p-3 text-center w-full overflow-hidden">
      <div class="mb-1 flex h-7 w-7 items-center justify-center rounded-full bg-gray-800/80 text-gray-500 shrink-0">
        <span class="icon-[heroicons--bookmark] h-3.5 w-3.5"></span>
      </div>
      <p class="text-xs font-medium text-gray-400 whitespace-nowrap overflow-hidden">No saved rune pages</p>
    </div>
  {:else}
    <ul class="space-y-2 w-full min-w-0">
      {#each items as item (item.id)}
        {@const isSelected = selectedKey === `saved:${item.id}`}
        {@const isActive = activeCompanionName === item.rune_page_name}

        <SavedRuneItem
          {item}
          {isSelected}
          {isActive}
          {allChampions}
          {layoutTransition}
          {onSelect}
          {onDelete}
          {onChangeChampion}
        />
      {/each}
    </ul>
  {/if}
</div>
