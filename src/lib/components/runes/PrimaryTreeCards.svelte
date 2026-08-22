<script lang="ts">
  import RuneCard from "./RuneCard.svelte";
  import type { TreePerksStats } from "../../types/rune_builder";
  import { getRuneTheme } from "../../utils/theme";

  let {
    tree,
    selectedPerkIds = [],
    onSelectPerk,
  }: {
    tree: TreePerksStats | null;
    selectedPerkIds: number[];
    onSelectPerk: (perkId: number, rowIndex: number) => void;
  } = $props();

  const theme = $derived(getRuneTheme(tree?.style_id));
</script>

{#if tree}
  <div class="flex flex-col gap-4 rounded-2xl border {theme.borderSubtle} bg-gray-950/80 p-4 shadow-md {theme.shadowGlow}">
    <!-- Primary Style Header -->
    <div class="flex items-center justify-between gap-3 border-b border-gray-800/80 pb-3">
      <div class="flex items-center gap-3">
        <div class="flex h-9 w-9 shrink-0 items-center justify-center rounded-xl border {theme.borderSubtle} {theme.bgCard} p-1.5 shadow-sm">
          {#if tree.icon_url}
            <img src={tree.icon_url} alt={tree.style_name} class="h-full w-full object-contain" />
          {/if}
        </div>
        <div>
          <p class="text-[10px] font-bold uppercase tracking-wider text-gray-400">Primary Tree</p>
          <h4 class="text-sm font-extrabold {theme.textAccent}">{tree.style_name}</h4>
        </div>
      </div>
      <span class="inline-flex items-center rounded-full border {theme.borderSubtle} {theme.bgCard} px-2.5 py-0.5 text-[10px] font-bold {theme.textAccent}">
        3 Perks Required
      </span>
    </div>

    <!-- 3 Tier Rows with Individual Cards -->
    <div class="flex flex-col gap-3.5">
      {#each tree.rows as row, rowIndex (rowIndex)}
        <div class="flex flex-col gap-1.5">
          <div class="flex items-center justify-between px-1">
            <span class="text-[11px] font-bold uppercase tracking-wider text-gray-400">
              {row.label || `Tier ${rowIndex + 1}`}
            </span>
          </div>

          <div class="grid grid-cols-1 sm:grid-cols-3 gap-2.5">
            {#each row.options as option (option.id)}
              <RuneCard
                option={{
                  ...option,
                  selected: selectedPerkIds.includes(option.id),
                }}
                {theme}
                onSelect={() => onSelectPerk(option.id, rowIndex)}
              />
            {/each}
          </div>
        </div>
      {/each}
    </div>
  </div>
{/if}
