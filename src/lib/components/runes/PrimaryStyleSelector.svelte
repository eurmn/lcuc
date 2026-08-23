<script lang="ts">
  import { getRuneTheme, type RuneTheme } from "../../utils/theme";

  let {
    primaryStyleId = 8000,
    theme,
    isEditing = true,
    onSelectPrimaryStyle,
  }: {
    primaryStyleId: number;
    theme: RuneTheme;
    isEditing?: boolean;
    onSelectPrimaryStyle: (styleId: number) => void;
  } = $props();

  // 5 League Style Paths in standard order
  const PRIMARY_STYLES = [
    { id: 8000, name: "Precision", icon: "https://raw.communitydragon.org/latest/plugins/rcp-be-lol-game-data/global/default/v1/perk-images/styles/7201_precision.png" },
    { id: 8100, name: "Domination", icon: "https://raw.communitydragon.org/latest/plugins/rcp-be-lol-game-data/global/default/v1/perk-images/styles/7200_domination.png" },
    { id: 8200, name: "Sorcery", icon: "https://raw.communitydragon.org/latest/plugins/rcp-be-lol-game-data/global/default/v1/perk-images/styles/7202_sorcery.png" },
    { id: 8300, name: "Inspiration", icon: "https://raw.communitydragon.org/latest/plugins/rcp-be-lol-game-data/global/default/v1/perk-images/styles/7203_whimsy.png" },
    { id: 8400, name: "Resolve", icon: "https://raw.communitydragon.org/latest/plugins/rcp-be-lol-game-data/global/default/v1/perk-images/styles/7204_resolve.png" },
  ];
</script>

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
