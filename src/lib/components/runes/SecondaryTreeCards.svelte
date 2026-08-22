<script lang="ts">
  import RuneCard from "./RuneCard.svelte";
  import type { SecondaryTreeOption, TreePerksStats } from "../../types/rune_builder";
  import { getRuneTheme } from "../../utils/theme";

  let {
    tree,
    candidates = [],
    selectedSubStyleId = 0,
    selectedPerkIds = [],
    onSelectSubStyle,
    onSelectSecondaryPerk,
  }: {
    tree: TreePerksStats | null;
    candidates: SecondaryTreeOption[];
    selectedSubStyleId: number;
    selectedPerkIds: number[];
    onSelectSubStyle: (styleId: number) => void;
    onSelectSecondaryPerk: (perkId: number) => void;
  } = $props();

  const theme = $derived(getRuneTheme(tree?.style_id));
</script>

<div class="flex flex-col gap-4 rounded-2xl border {theme.borderSubtle} bg-gray-950/80 p-4 shadow-md {theme.shadowGlow}">
  <!-- Secondary Style Header & Popularity Tabs -->
  <div class="flex flex-col gap-2.5 border-b border-gray-800/80 pb-3">
    <div class="flex items-center justify-between">
      <div class="flex items-center gap-2">
        <p class="text-[10px] font-bold uppercase tracking-wider text-gray-400">Secondary Tree</p>
        <span class="inline-flex items-center rounded-full border border-gray-700 bg-gray-900 px-2 py-0.5 text-[10px] font-bold text-gray-300">
          Pick 2 Perks
        </span>
      </div>
      {#if tree}
        <h4 class="text-xs font-extrabold {theme.textAccent}">{tree.style_name}</h4>
      {/if}
    </div>

    <!-- Candidate Secondary Tree Tabs with Popularity % Badges -->
    <div class="grid grid-cols-2 sm:grid-cols-4 gap-2">
      {#each candidates as cand (cand.style_id)}
        {@const candTheme = getRuneTheme(cand.style_id)}
        <button
          type="button"
          onclick={() => onSelectSubStyle(cand.style_id)}
          class={selectedSubStyleId === cand.style_id
            ? `flex items-center gap-2 rounded-xl border-2 ${candTheme.borderPrimary} ${candTheme.bgCard} p-2 shadow-md ${candTheme.shadowGlow} ring-1 ${candTheme.ringActive} cursor-pointer`
            : "flex items-center gap-2 rounded-xl border border-gray-800 bg-gray-900/90 p-2 opacity-60 hover:opacity-100 hover:border-gray-700 hover:bg-gray-800 cursor-pointer transition-all"}
        >
          <div class="flex h-6 w-6 shrink-0 items-center justify-center rounded-lg bg-gray-950 p-1 border border-gray-800">
            {#if cand.icon_url}
              <img src={cand.icon_url} alt={cand.style_name} class="h-full w-full object-contain" />
            {/if}
          </div>
          <div class="flex flex-col min-w-0 text-left">
            <span class="text-[11px] font-bold text-gray-100 truncate">{cand.style_name}</span>
            <span class="text-[10px] font-extrabold {cand.is_recommended ? 'text-emerald-400' : 'text-gray-400'}">
              {cand.playcount_pct.toFixed(1)}% pick
            </span>
          </div>
        </button>
      {/each}
    </div>
  </div>

  <!-- 3 Tier Rows for Selected Secondary Style -->
  {#if tree}
    <div class="flex flex-col gap-3">
      {#each tree.rows as row, rowIndex (rowIndex)}
        <div class="grid grid-cols-1 sm:grid-cols-3 gap-2">
          {#each row.options as option (option.id)}
            <RuneCard
              option={{
                ...option,
                selected: selectedPerkIds.includes(option.id),
              }}
              {theme}
              onSelect={() => onSelectSecondaryPerk(option.id)}

            />
          {/each}
        </div>
      {/each}
    </div>
  {/if}
</div>
