<script lang="ts">
  import PrimaryStyleSelector from "./PrimaryStyleSelector.svelte";
  import KeystonesRow from "./KeystonesRow.svelte";
  import PerkTierList from "./PerkTierList.svelte";
  import type { EnrichedKeystoneOption, TreePerksStats } from "../../types/rune_builder";
  import { getRuneTheme } from "../../utils/theme";

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

  // Only show keystones belonging to the selected primary tree
  const visibleKeystones = $derived(
    keystones.filter((k) => k.style_id === primaryStyleId)
  );
</script>

<div class="flex flex-col gap-4 rounded-2xl border border-gray-800 bg-gray-950/80 p-4 shadow-md">
  <!-- 1. Primary Style Selector Row (5 Style Icons) -->
  <PrimaryStyleSelector
    {primaryStyleId}
    {theme}
    {isEditing}
    {onSelectPrimaryStyle}
  />

  <!-- 2. Keystones Section (Only for Selected Primary Tree, In-Game Natural Order) -->
  <KeystonesRow
    {visibleKeystones}
    {selectedKeystoneId}
    {selectedPerkIds}
    {theme}
    {isEditing}
    {onSelectKeystone}
  />

  <!-- 3. Primary Tree 3 Tiers (With Authentic Row Names) -->
  <PerkTierList
    {tree}
    {primaryStyleId}
    {selectedPerkIds}
    {theme}
    {isEditing}
    {onSelectPerk}
  />
</div>

