<script lang="ts">
  import RuneCard from "./RuneCard.svelte";
  import KeystoneCard from "./KeystoneCard.svelte";
  import type { EnrichedKeystoneOption, TreePerksStats } from "../../types/rune_builder";
  import { getRuneTheme, getStyleRowName } from "../../utils/theme";

  let {
    primaryStyleId = 8000,
    keystones = [],
    selectedKeystoneId = null,
    tree = null,
    selectedPerkIds = [],
    isEditing = true,
    onSelectPrimaryStyle,
    onSelectKeystone,
    onSelectPerk,
  }: {
    primaryStyleId: number;
    keystones: EnrichedKeystoneOption[];
    selectedKeystoneId: number | null;
    tree: TreePerksStats | null;
    selectedPerkIds: number[];
    isEditing?: boolean;
    onSelectPrimaryStyle: (styleId: number) => void;
    onSelectKeystone: (keystoneId: number) => void;
    onSelectPerk: (perkId: number, rowIndex: number) => void;
  } = $props();

  const theme = $derived(getRuneTheme(primaryStyleId));

  // 5 League Style Paths in standard order
  const PRIMARY_STYLES = [
    { id: 8000, name: "Precision", icon: "https://raw.communitydragon.org/latest/plugins/rcp-be-lol-game-data/global/default/v1/perk-images/styles/7201_precision.png" },
    { id: 8100, name: "Domination", icon: "https://raw.communitydragon.org/latest/plugins/rcp-be-lol-game-data/global/default/v1/perk-images/styles/7200_domination.png" },
    { id: 8200, name: "Sorcery", icon: "https://raw.communitydragon.org/latest/plugins/rcp-be-lol-game-data/global/default/v1/perk-images/styles/7202_sorcery.png" },
    { id: 8300, name: "Inspiration", icon: "https://raw.communitydragon.org/latest/plugins/rcp-be-lol-game-data/global/default/v1/perk-images/styles/7203_whimsy.png" },
    { id: 8400, name: "Resolve", icon: "https://raw.communitydragon.org/latest/plugins/rcp-be-lol-game-data/global/default/v1/perk-images/styles/7204_resolve.png" },
  ];

  // Only show keystones belonging to the selected primary tree
  const visibleKeystones = $derived(
    keystones.filter((k) => k.style_id === primaryStyleId)
  );
</script>

<div class="flex flex-col gap-4 rounded-2xl border border-gray-800 bg-gray-950/80 p-4 shadow-md">
  <!-- 1. Primary Style Selector Row (5 Style Icons) -->
  <div class="flex items-center justify-between border-b border-gray-800/80 pb-3">
    <div class="flex items-center gap-2">
      <span class="flex h-2 w-2 rounded-full {theme.bgBadge}"></span>
      <h3 class="text-xs font-bold uppercase tracking-wider text-gray-300">Primary Tree</h3>
    </div>

    <!-- 5 Style Icon Buttons -->
    <div class="flex items-center gap-1.5">
      {#each PRIMARY_STYLES as style (style.id)}
        {@const styleTheme = getRuneTheme(style.id)}
        {@const isSelected = primaryStyleId === style.id}
        <button
          type="button"
          title={style.name}
          disabled={!isEditing}
          onclick={() => isEditing && onSelectPrimaryStyle(style.id)}
          class={isSelected
            ? (isEditing
              ? `flex h-9 w-9 items-center justify-center rounded-xl border-2 ${styleTheme.borderPrimary} bg-gray-900 p-1 shadow-sm scale-105 cursor-pointer`
              : `flex h-9 w-9 items-center justify-center rounded-xl border-2 ${styleTheme.borderPrimary} bg-gray-900 p-1 shadow-sm cursor-default`)
            : (isEditing
              ? "flex h-9 w-9 items-center justify-center rounded-xl border border-gray-800 bg-gray-900/60 p-1.5 opacity-50 hover:opacity-100 hover:border-gray-700 hover:bg-gray-800 transition-all cursor-pointer"
              : "flex h-9 w-9 items-center justify-center rounded-xl border border-gray-800/60 bg-gray-900/40 p-1.5 opacity-35 cursor-default")}
        >
          <img src={style.icon} alt={style.name} class="h-full w-full object-contain" />
        </button>
      {/each}
    </div>
  </div>

  <!-- 2. Keystones Section (Only for Selected Primary Tree, In-Game Natural Order) -->
  <div class="flex flex-col gap-2">
    <div class="flex items-center justify-between px-0.5">
      <span class="text-[11px] font-extrabold uppercase tracking-wider {theme.textAccent}">
        {theme.name} Keystones
      </span>
    </div>

    <div
      class="grid gap-2 w-full min-w-0"
      style="grid-template-columns: repeat({visibleKeystones.length}, minmax(0, 1fr));"
    >
      {#each visibleKeystones as keystone (keystone.id)}
        <KeystoneCard
          keystone={{
            ...keystone,
            selected: (selectedKeystoneId === keystone.id) || selectedPerkIds[0] === keystone.id,
          }}
          {isEditing}
          onSelect={(id) => isEditing && onSelectKeystone(id)}
        />
      {/each}
    </div>
  </div>

  <!-- 3. Primary Tree 3 Tiers (With Authentic Row Names) -->
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
</div>
