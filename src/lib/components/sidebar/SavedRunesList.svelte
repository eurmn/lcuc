<script lang="ts">
  import type { RuneSetSummary } from "../../types/runes";
  import { ALL_CHAMPIONS_ICON_URL } from "../../types/champions";
  import { getRuneTheme } from "../../utils/theme";

  let {
    items = [],
    selectedKey = "",
    activeCompanionName = null,
    onSelect,
    onDelete,
  }: {
    items: RuneSetSummary[];
    selectedKey: string;
    activeCompanionName: string | null;
    onSelect: (runesetId: number) => void;
    onDelete: (runesetId: number) => void;
  } = $props();
</script>

<div>
  <div class="mb-3 flex items-center justify-between px-1">
    <div class="flex items-center gap-2">
      <span class="flex h-2 w-2 rounded-full bg-purple-500 shadow-sm shadow-purple-500"></span>
      <h3 class="text-xs font-bold uppercase tracking-wider text-gray-300">Companion Saved Runes</h3>
    </div>
    <span class="rounded-full bg-gray-800 px-2 py-0.5 text-[11px] font-semibold text-gray-400 border border-gray-700/60">
      {items.length}
    </span>
  </div>

  {#if items.length === 0}
    <div class="flex flex-col items-center justify-center rounded-2xl border border-dashed border-gray-800 bg-gray-900/30 p-5 text-center">
      <div class="mb-2 flex h-8 w-8 items-center justify-center rounded-full bg-gray-800/80 text-gray-500">
        <span class="icon-[heroicons--bookmark] h-4 w-4"></span>
      </div>
      <p class="text-xs font-medium text-gray-400">No saved rune pages</p>
      <p class="mt-1 text-[11px] text-gray-500">
        Click <span class="font-medium text-purple-400">Import</span> on any client page to save it
      </p>
    </div>
  {:else}
    <ul class="space-y-2">
      {#each items as item (item.id)}
        {@const isSelected = selectedKey === `saved:${item.id}`}
        {@const isActive = activeCompanionName === item.rune_page_name}
        {@const theme = getRuneTheme(item.primary_style_id)}
        {@const styleOrSource = item.primary_style_name || item.source}
        {@const createdDate = item.created_at.length >= 10 ? item.created_at.slice(0, 10) : item.created_at}

        <li
          class={isSelected
            ? `group relative flex items-center justify-between gap-3 rounded-2xl border-2 ${theme.borderPrimary} bg-gradient-to-r ${theme.cardGradient} p-3 ring-1 ${theme.borderSubtle} shadow-md ${theme.shadowGlow} cursor-pointer`
            : "group relative flex items-center justify-between gap-3 rounded-2xl border border-gray-800/80 bg-gray-900/70 p-3 transition-all duration-200 hover:border-gray-700 hover:bg-gray-800/80 hover:shadow-md hover:shadow-black/30 cursor-pointer"}
        >
          {#if isSelected}
            <div class="absolute -left-1 top-1/2 -translate-y-1/2 h-6 w-1 rounded-r-full {theme.activeBar}"></div>
          {/if}

          <button
            type="button"
            onclick={() => onSelect(item.id)}
            class="flex min-w-0 flex-1 items-center gap-2.5 text-left focus:outline-none cursor-pointer"
          >
            <!-- Keystone Style Icon -->
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

            <!-- Champion Icon on the Right of Keystone Icon -->
            <div
              class="relative flex h-8 w-8 shrink-0 items-center justify-center overflow-hidden rounded-full border border-gray-700/80 bg-gray-950 p-0 shadow-inner"
            >
              {#if item.champion_id && item.champion_id > 0}
                <img
                  src="https://raw.communitydragon.org/latest/plugins/rcp-be-lol-game-data/global/default/v1/champion-icons/{item.champion_id}.png"
                  alt="Champion"
                  class="absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 h-[calc(100%+4px)] w-[calc(100%+4px)] object-cover rounded-full"
                />
              {:else}
                <img
                  src={ALL_CHAMPIONS_ICON_URL}
                  alt="All Champions"
                  class="opacity-80 w-[70%] h-[70%] object-contain"
                />
              {/if}
            </div>

            <!-- Title & Info Column -->
            <div class="min-w-0 flex-1">
              <div class="flex items-center gap-1.5">
                <p
                  class={isSelected
                    ? `truncate text-sm font-bold ${theme.textAccent}`
                    : "truncate text-sm font-semibold text-gray-200 group-hover:text-white transition"}
                >
                  {item.rune_page_name}
                </p>
                {#if isActive}
                  <span class="inline-flex items-center gap-1 rounded-full border border-purple-500/40 bg-purple-500/15 px-2 py-0.5 text-[10px] font-bold uppercase tracking-wider text-purple-300">
                    <span class="h-1.5 w-1.5 rounded-full bg-purple-400 animate-pulse"></span>
                    Active
                  </span>
                {/if}
              </div>

              <div class="flex items-center gap-2 mt-0.5">
                <span class="inline-flex items-center rounded-md px-1.5 py-0.5 text-[10px] font-semibold {theme.badgeClasses}">
                  {styleOrSource}
                </span>
                <span class="text-[11px] text-gray-500">{createdDate}</span>
              </div>
            </div>
          </button>

          <button
            type="button"
            title="Delete from database"
            onclick={(e) => {
              e.stopPropagation();
              onDelete(item.id);
            }}
            class="inline-flex shrink-0 items-center justify-center rounded-xl p-2 text-gray-500 opacity-0 transition-all hover:bg-red-500/10 hover:text-red-400 hover:border-red-500/30 border border-transparent focus:outline-none group-hover:opacity-100 cursor-pointer"
          >
            <span class="icon-[heroicons--trash] h-4 w-4"></span>
          </button>
        </li>
      {/each}
    </ul>
  {/if}
</div>
