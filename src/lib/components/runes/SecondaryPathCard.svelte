<script lang="ts">
  import SecondaryStyleSelector from "./SecondaryStyleSelector.svelte";
  import SecondaryTierList from "./SecondaryTierList.svelte";
  import StatShardsCards from "./StatShardsCards.svelte";
  import type { SecondaryTreeOption, StatShardsStats, TreePerksStats } from "../../types/rune_builder";
  import { getRuneTheme } from "../../utils/theme";

  let {
    primaryStyleId = 8000,
    secondaryStyleId = 8100,
    candidates = [],
    tree = null,
    statShards,
    selectedPerkIds = [],
    isEditing = true,
    onSelectSubStyle,
    onSelectSecondaryPerk,
    onSelectShard,
  }: {
    primaryStyleId: number;
    secondaryStyleId: number;
    candidates: SecondaryTreeOption[];
    tree: TreePerksStats | null;
    statShards: StatShardsStats;
    selectedPerkIds: number[];
    isEditing?: boolean;
    onSelectSubStyle: (styleId: number) => void;
    onSelectSecondaryPerk: (perkId: number, rowIndex: number) => void;
    onSelectShard: (shardId: number, rowIndex: number) => void;
  } = $props();

  const theme = $derived(getRuneTheme(secondaryStyleId));
</script>

<div class="flex flex-col gap-4 rounded-2xl border border-gray-800 bg-gray-900 p-4 shadow-md">
  <!-- 1. Secondary Tree Style Selector Row (4 Style Icons with Popularity %) -->
  <SecondaryStyleSelector
    {primaryStyleId}
    {secondaryStyleId}
    {candidates}
    {theme}
    {isEditing}
    {onSelectSubStyle}
  />

  <!-- 2. Secondary 3 Tiers (With Authentic Row Names) -->
  <SecondaryTierList
    {tree}
    {secondaryStyleId}
    {selectedPerkIds}
    {theme}
    {isEditing}
    {onSelectSecondaryPerk}
  />

  <!-- 3. Stat Shards Section -->
  <div class="border-t border-gray-800/80 pt-2">
    <StatShardsCards
      {statShards}
      {isEditing}
      selectedShardIds={selectedPerkIds.slice(6, 9)}
      onSelectShard={(id, row) => isEditing && onSelectShard(id, row)}
    />
  </div>
</div>

