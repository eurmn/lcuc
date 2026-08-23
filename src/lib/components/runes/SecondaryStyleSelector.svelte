<script lang="ts">
  import type { SecondaryTreeOption } from "../../types/rune_builder";
  import { getRuneTheme, type RuneTheme } from "../../utils/theme";

  let {
    primaryStyleId = 8000,
    secondaryStyleId = 8100,
    candidates = [],
    theme,
    isEditing = true,
    onSelectSubStyle,
  }: {
    primaryStyleId: number;
    secondaryStyleId: number;
    candidates: SecondaryTreeOption[];
    theme: RuneTheme;
    isEditing?: boolean;
    onSelectSubStyle: (styleId: number) => void;
  } = $props();

  // 5 League Style Paths in standard order
  const ALL_STYLES = [
    { id: 8000, name: "Precision", icon: "https://raw.communitydragon.org/latest/plugins/rcp-be-lol-game-data/global/default/v1/perk-images/styles/7201_precision.png" },
    { id: 8100, name: "Domination", icon: "https://raw.communitydragon.org/latest/plugins/rcp-be-lol-game-data/global/default/v1/perk-images/styles/7200_domination.png" },
    { id: 8200, name: "Sorcery", icon: "https://raw.communitydragon.org/latest/plugins/rcp-be-lol-game-data/global/default/v1/perk-images/styles/7202_sorcery.png" },
    { id: 8300, name: "Inspiration", icon: "https://raw.communitydragon.org/latest/plugins/rcp-be-lol-game-data/global/default/v1/perk-images/styles/7203_whimsy.png" },
    { id: 8400, name: "Resolve", icon: "https://raw.communitydragon.org/latest/plugins/rcp-be-lol-game-data/global/default/v1/perk-images/styles/7204_resolve.png" },
  ];

  // Exclude current primary style
  const secondaryStyles = $derived(ALL_STYLES.filter((s) => s.id !== primaryStyleId));

  function getPlaycountBadge(styleId: number): string | null {
    const cand = candidates.find((c) => c.style_id === styleId);
    return cand ? `${cand.playcount_pct.toFixed(0)}%` : null;
  }
</script>

<div class="flex items-center justify-between border-b border-gray-800/80 pb-3">
  <div class="flex items-center gap-2">
    <span class="flex h-2 w-2 rounded-full {theme.bgBadge}"></span>
    <h3 class="text-xs font-bold uppercase tracking-wider text-gray-300">Secondary Tree</h3>
  </div>

  <!-- 4 Secondary Style Icon Buttons -->
  <div class="flex items-center gap-2">
    {#each secondaryStyles as style (style.id)}
      {@const styleTheme = getRuneTheme(style.id)}
      {@const isSelected = secondaryStyleId === style.id}
      {@const pct = getPlaycountBadge(style.id)}
      <button
        type="button"
        title="{style.name} {pct ? `(${pct} pick rate)` : ''}"
        disabled={!isEditing}
        onclick={() => isEditing && onSelectSubStyle(style.id)}
        class={isSelected
          ? (isEditing
            ? `relative flex h-9 min-w-[42px] items-center justify-center gap-1 rounded-xl border-2 ${styleTheme.borderPrimary} bg-gray-900 p-1 shadow-sm scale-105 cursor-pointer`
            : `relative flex h-9 min-w-[42px] items-center justify-center gap-1 rounded-xl border-2 ${styleTheme.borderPrimary} bg-gray-900 p-1 shadow-sm cursor-default`)
          : (isEditing
            ? "relative flex h-9 min-w-[42px] items-center justify-center gap-1 rounded-xl border border-gray-800 bg-gray-900/60 p-1 opacity-50 hover:opacity-100 hover:border-gray-700 hover:bg-gray-800 transition-all cursor-pointer"
            : "relative flex h-9 min-w-[42px] items-center justify-center gap-1 rounded-xl border border-gray-800/60 bg-gray-900/40 p-1 opacity-35 cursor-default")}
      >
        <img src={style.icon} alt={style.name} class="h-6 w-6 object-contain" />
        {#if pct}
          <span class="text-[9px] font-extrabold {isSelected ? styleTheme.textAccent : 'text-gray-400'} pr-0.5">
            {pct}
          </span>
        {/if}
      </button>
    {/each}
  </div>
</div>
