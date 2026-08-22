<script lang="ts">
  import type { ClientRunePageSummary } from "../../types/runes";
  import { ALL_CHAMPIONS_ICON_URL } from "../../types/champions";
  import { getRuneTheme } from "../../utils/theme";

  let {
    items = [],
    selectedKey = "",
    onSelect,
    onImport,
  }: {
    items: ClientRunePageSummary[];
    selectedKey: string;
    onSelect: (pageId: number) => void;
    onImport: (pageId: number) => void;
  } = $props();
</script>

<div>
  <div class="mb-3 flex items-center justify-between px-1">
    <div class="flex items-center gap-2">
      <span class="flex h-2 w-2 rounded-full bg-blue-500 shadow-sm shadow-blue-500"></span>
      <h3 class="text-xs font-bold uppercase tracking-wider text-gray-300">League Client Pages</h3>
    </div>
    <span class="rounded-full bg-gray-800 px-2 py-0.5 text-[11px] font-semibold text-gray-400 border border-gray-700/60">
      {items.length}
    </span>
  </div>

  {#if items.length === 0}
    <div class="flex flex-col items-center justify-center rounded-2xl border border-dashed border-gray-800 bg-gray-900/30 p-5 text-center">
      <div class="mb-2 flex h-8 w-8 items-center justify-center rounded-full bg-gray-800/80 text-gray-500">
        <span class="icon-[heroicons--exclamation-circle] h-4 w-4"></span>
      </div>
      <p class="text-xs font-medium text-gray-400">No client pages detected</p>
      <p class="mt-1 text-[11px] text-gray-500">Launch League of Legends to view pages</p>
    </div>
  {:else}
    <ul class="space-y-2">
      {#each items as item (item.id)}
        {@const isSelected = selectedKey === `client:${item.id}`}
        {@const theme = getRuneTheme(item.primary_style_id)}

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
            <!-- Primary Style Icon -->
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

            <!-- All Champions / Autofill Icon on the Right of Keystone Icon -->
            <div
              class="relative flex h-8 w-8 shrink-0 items-center justify-center overflow-hidden rounded-full border border-gray-700/80 bg-gray-950 p-0 shadow-inner"
            >
              <img
                src={ALL_CHAMPIONS_ICON_URL}
                alt="All Champions"
                class="opacity-80 w-[70%] h-[70%] object-contain"
              />
            </div>

            <div class="min-w-0 flex-1">
              <div class="flex items-center gap-1.5">
                <p
                  class={isSelected
                    ? `truncate text-sm font-bold ${theme.textAccent}`
                    : "truncate text-sm font-semibold text-gray-200 group-hover:text-white transition"}
                >
                  {item.name}
                </p>
                {#if item.is_active}
                  <span class="inline-flex items-center gap-1 rounded-full border border-emerald-500/30 bg-emerald-500/10 px-2 py-0.5 text-[10px] font-semibold text-emerald-400">
                    <span class="h-1.5 w-1.5 rounded-full bg-emerald-400 animate-pulse"></span>
                    Active
                  </span>
                {/if}
              </div>

              <div class="mt-0.5">
                <span class="inline-flex items-center rounded-md px-1.5 py-0.5 text-[10px] font-semibold {theme.badgeClasses}">
                  {item.primary_style_name}
                </span>
              </div>
            </div>
          </button>

          <button
            type="button"
            title="Import into Companion database"
            onclick={(e) => {
              e.stopPropagation();
              onImport(item.id);
            }}
            class="inline-flex shrink-0 items-center gap-1.5 rounded-xl border border-purple-500/30 bg-purple-500/10 px-2.5 py-1.5 text-xs font-semibold text-purple-300 transition-all hover:border-purple-500 hover:bg-purple-600 hover:text-white focus:outline-none focus:ring-2 focus:ring-purple-500/40 shadow-sm cursor-pointer"
          >
            <span class="icon-[heroicons--arrow-down-tray] h-3.5 w-3.5"></span>
            <span>Import</span>
          </button>
        </li>
      {/each}
    </ul>
  {/if}
</div>
