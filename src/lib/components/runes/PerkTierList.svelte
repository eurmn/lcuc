<script lang="ts">
  import RuneCard from "./RuneCard.svelte";
  import type { TreePerksStats } from "../../types/rune_builder";
  import { getStyleRowName, type RuneTheme } from "../../utils/theme";

  let {
    tree,
    primaryStyleId = 8000,
    selectedPerkIds = [],
    theme,
    isEditing = true,
    onSelectPerk,
  }: {
    tree: TreePerksStats | null;
    primaryStyleId: number;
    selectedPerkIds: number[];
    theme: RuneTheme;
    isEditing?: boolean;
    onSelectPerk: (perkId: number, rowIndex: number) => void;
  } = $props();
</script>

{#if tree}
  <div class="flex flex-col gap-3 pt-2 border-t border-gray-800/60">
    {#each tree.rows as row, rowIndex (rowIndex)}
      <div class="flex flex-col gap-1">
        <span class="text-[10px] font-bold uppercase tracking-wider text-gray-400 px-0.5">
          {getStyleRowName(primaryStyleId, rowIndex)}
        </span>

        <div class="grid grid-cols-3 gap-2">
          {#each row.options as option (option.id)}
            <RuneCard
              option={{
                ...option,
                selected: selectedPerkIds.includes(option.id),
              }}
              {theme}
              {isEditing}
              onSelect={(id) => isEditing && onSelectPerk(id, rowIndex)}
            />
          {/each}
        </div>
      </div>
    {/each}
  </div>
{/if}
