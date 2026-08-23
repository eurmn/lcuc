<script lang="ts">
  import { MotionLi, MotionSpan } from "@humanspeak/svelte-motion";
  import type { RuneSetSummary } from "../../types/runes";
  import type { ChampionSummary } from "../../types/champions";
  import { ALL_CHAMPIONS_ICON_URL } from "../../types/champions";
  import { getRuneTheme } from "../../utils/theme";

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

  function getChampName(champId: number): string {
    if (!champId || champId === 0) return "All";
    const found = allChampions.find((c) => c.id === champId);
    return found ? found.name : "Custom";
  }

  let layoutTransition = $derived({
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
        {@const theme = getRuneTheme(item.primary_style_id)}
        {@const styleOrSource = item.primary_style_name || item.source}
        {@const champName = getChampName(item.champion_id)}

        <MotionLi
          layout="position"
          transition={layoutTransition}
          class={isSelected
            ? `group relative flex items-center justify-between gap-2.5 rounded-2xl border-2 ${theme.borderPrimary} bg-gradient-to-r ${theme.cardGradient} p-2.5 ring-1 ${theme.borderSubtle} shadow-md ${theme.shadowGlow} cursor-pointer overflow-hidden w-full min-w-0`
            : "group relative flex items-center justify-between gap-2.5 rounded-2xl border border-gray-800/80 bg-gray-900/70 p-2.5 transition-colors duration-200 hover:border-gray-700 hover:bg-gray-800/80 hover:shadow-md hover:shadow-black/30 cursor-pointer overflow-hidden w-full min-w-0"}
          onclick={() => onSelect(item.id)}
        >
          {#if isSelected}
            <div class="absolute -left-1 top-1/2 -translate-y-1/2 h-6 w-1 rounded-r-full {theme.activeBar}"></div>
          {/if}

          <!-- 1. Keystone Style Icon (Always visible on left) -->
          <div
            class={isSelected
              ? `relative flex h-10 w-10 shrink-0 items-center justify-center rounded-xl border p-1.5 shadow-sm ring-1 ${theme.ringActive} ${theme.iconBoxClasses}`
              : `relative flex h-10 w-10 shrink-0 items-center justify-center rounded-xl border p-1.5 shadow-sm group-hover:brightness-110 transition ${theme.iconBoxClasses}`}
          >
            {#if item.primary_style_icon_url}
              <img
                src={item.primary_style_icon_url}
                alt={item.primary_style_name}
                class="h-full w-full object-contain"
              />
            {:else}
              <div class="h-full w-full rounded-lg bg-gray-800"></div>
            {/if}
          </div>

          <!-- 2. Champion Icon / Avatar (Next to Keystone) -->
          <button
            type="button"
            title="Click to change champion for this rune page"
            onclick={(e) => {
              e.stopPropagation();
              onChangeChampion?.(item.id, item.champion_id);
            }}
            class="group/champ relative flex h-7 w-7 shrink-0 items-center justify-center overflow-hidden rounded-full border border-gray-700/80 bg-gray-950 p-0 shadow-inner hover:border-purple-500 hover:scale-110 cursor-pointer transition-all"
          >
            {#if item.champion_id && item.champion_id > 0}
              <img
                src="https://raw.communitydragon.org/latest/plugins/rcp-be-lol-game-data/global/default/v1/champion-icons/{item.champion_id}.png"
                alt="Champion"
                class="absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 h-[calc(100%+4px)] w-[calc(100%+4px)] object-cover rounded-full group-hover/champ:brightness-50 transition-all"
              />
            {:else}
              <img
                src={ALL_CHAMPIONS_ICON_URL}
                alt="All Champions"
                class="opacity-80 w-[70%] h-[70%] object-contain group-hover/champ:brightness-50 transition-all"
              />
            {/if}

            <div class="absolute inset-0 bg-black/60 rounded-full flex items-center justify-center opacity-0 group-hover/champ:opacity-100 transition-opacity">
              <span class="icon-[heroicons--pencil] h-3 w-3 text-purple-300"></span>
            </div>
          </button>

          <!-- 3. Title & Info Column (Progressively eaten by overflow as sidebar shrinks) -->
          <div class="min-w-0 flex-1 text-left overflow-hidden whitespace-nowrap">
            <div class="flex items-center gap-1.5 overflow-hidden whitespace-nowrap">
              <p
                class={isSelected
                  ? `truncate text-sm font-bold ${theme.textAccent}`
                  : "truncate text-sm font-semibold text-gray-200 group-hover:text-white transition"}
              >
                {item.rune_page_name}
              </p>
              {#if isActive}
                <span class="inline-flex items-center gap-1 rounded-full border border-purple-500/40 bg-purple-500/15 px-1.5 py-0.5 text-[9px] font-bold uppercase tracking-wider text-purple-300 shrink-0">
                  <span class="h-1.5 w-1.5 rounded-full bg-purple-400 animate-pulse"></span>
                  Active
                </span>
              {/if}
            </div>

            <div class="flex items-center gap-2 mt-0.5 overflow-hidden whitespace-nowrap">
              <span class="inline-flex items-center rounded-md px-1.5 py-0.5 text-[10px] font-semibold {theme.badgeClasses} shrink-0">
                {styleOrSource}
              </span>
              <span class="text-[11px] text-gray-500 shrink-0">{champName}</span>
            </div>
          </div>

          <!-- 4. Delete Action Button (Clipped away as width decreases) -->
          <button
            type="button"
            title="Delete from database"
            onclick={(e) => {
              e.stopPropagation();
              onDelete(item.id);
            }}
            class="inline-flex shrink-0 items-center justify-center rounded-xl p-1.5 text-gray-500 opacity-0 transition-all hover:bg-red-500/10 hover:text-red-400 hover:border-red-500/30 border border-transparent focus:outline-none group-hover:opacity-100 cursor-pointer"
          >
            <span class="icon-[heroicons--trash] h-4 w-4"></span>
          </button>
        </MotionLi>
      {/each}
    </ul>
  {/if}
</div>
