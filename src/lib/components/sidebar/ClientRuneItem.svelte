<script lang="ts">
  import { MotionLi, type MotionTransition } from "@humanspeak/svelte-motion";
  import type { ClientRunePageSummary } from "../../types/runes";
  import { ALL_CHAMPIONS_ICON_URL } from "../../types/champions";
  import { getRuneTheme } from "../../utils/theme";

  let {
    item,
    isSelected = false,
    layoutTransition,
    onSelect,
    onImport,
  }: {
    item: ClientRunePageSummary;
    isSelected: boolean;
    layoutTransition: MotionTransition;
    onSelect: (pageId: number) => void;
    onImport: (pageId: number) => void;
  } = $props();

  const theme = $derived(getRuneTheme(item.primary_style_id));
</script>

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

  <!-- 1. Primary Style Icon (Always visible on left) -->
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

  <!-- 2. All Champions Icon -->
  <div
    class="relative flex h-7 w-7 shrink-0 items-center justify-center overflow-hidden rounded-full border border-gray-700/80 bg-gray-900 p-0 shadow-inner"
  >
    <img
      src={ALL_CHAMPIONS_ICON_URL}
      alt="All Champions"
      class="opacity-80 w-[70%] h-[70%] object-contain"
    />
  </div>

  <!-- 3. Title & Style (Progressively eaten by overflow) -->
  <div class="min-w-0 flex-1 overflow-hidden whitespace-nowrap text-left">
    <div class="flex items-center gap-1.5 overflow-hidden whitespace-nowrap">
      <p
        class={isSelected
          ? `truncate text-sm font-bold ${theme.textAccent}`
          : "truncate text-sm font-semibold text-gray-200 group-hover:text-white transition"}
      >
        {item.name}
      </p>
      {#if item.is_active}
        <span class="inline-flex items-center gap-1 rounded-full border border-emerald-500/30 bg-emerald-500/10 px-1.5 py-0.5 text-[9px] font-semibold text-emerald-400 shrink-0">
          <span class="h-1.5 w-1.5 rounded-full bg-emerald-400 animate-pulse"></span>
          Active
        </span>
      {/if}
    </div>

    <div class="mt-0.5 overflow-hidden whitespace-nowrap">
      <span class="inline-flex items-center rounded-md px-1.5 py-0.5 text-[10px] font-semibold {theme.badgeClasses} shrink-0">
        {item.primary_style_name}
      </span>
    </div>
  </div>

  <!-- 4. Import Button (Clipped away as width decreases) -->
  <button
    type="button"
    title="Import into Companion database"
    onclick={(e) => {
      e.stopPropagation();
      onImport(item.id);
    }}
    class="inline-flex shrink-0 items-center gap-1 rounded-xl border border-purple-500/30 bg-purple-500/10 px-2 py-1 text-xs font-semibold text-purple-300 transition-all hover:border-purple-500 hover:bg-purple-600 hover:text-white focus:outline-none focus:ring-2 focus:ring-purple-500/40 shadow-sm cursor-pointer"
  >
    <span class="icon-[heroicons--arrow-down-tray] h-3.5 w-3.5"></span>
    <span>Import</span>
  </button>
</MotionLi>
