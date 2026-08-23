<script lang="ts">
  import RuneCard from "./RuneCard.svelte";
  import type { TreePerksStats } from "../../types/rune_builder";
  import { getStyleRowName, type RuneTheme } from "../../utils/theme";

  let {
    tree,
    secondaryStyleId = 8100,
    selectedPerkIds = [],
    theme,
    isEditing = true,
    onSelectSecondaryPerk,
  }: {
    tree: TreePerksStats | null;
    secondaryStyleId: number;
    selectedPerkIds: number[];
    theme: RuneTheme;
    isEditing?: boolean;
    onSelectSecondaryPerk: (perkId: number, rowIndex: number) => void;
  } = $props();
</script>

{#if tree}
  <div class="flex flex-col gap-3">
    {#each tree.rows as row, rowIndex (rowIndex)}
      <div class="flex flex-col gap-1">
        <span class="text-[10px] font-bold uppercase tracking-wider text-gray-400 px-0.5">
          {getStyleRowName(secondaryStyleId, rowIndex)}
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
              onSelect={(id) => isEditing && onSelectSecondaryPerk(id, rowIndex)}
            />
          {/each}
        </div>
      </div>
    {/each}
  </div>
{/if}
