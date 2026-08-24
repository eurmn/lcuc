<script lang="ts">
  import { onMount } from "svelte";
  import RuneCanvasHeader from "./RuneCanvasHeader.svelte";
  import RuneLoadingSkeleton from "./RuneLoadingSkeleton.svelte";
  import PrimaryPathCard from "./PrimaryPathCard.svelte";
  import SecondaryPathCard from "./SecondaryPathCard.svelte";
  import ChampionSelectModal from "../ChampionSelectModal.svelte";
  import {
    getBuilderOverview,
    getBuilderTreeRunes,
  } from "../../api/rune_builder";
  import { getChampions } from "../../api/champions";

  import type { ChampionSummary } from "../../types/champions";
  import type {
    BuilderQuery,
    EnrichedKeystoneOption,
    RuneBuilderOverview,
    SecondaryTreeOption,
    StatShardsStats,
    TreePerksStats,
  } from "../../types/rune_builder";
  import type { RuneTreePayload } from "../../types/runes";

  let {
    tree = null,
    loading = false,
    selectedChampion = null,
    onSave,
    onExport,
    onSelectChampion,
  }: {
    tree: RuneTreePayload | null;
    loading: boolean;
    selectedChampion?: ChampionSummary | null;
    onSave: (payload: {
      name: string;
      primaryStyleId: number;
      subStyleId: number;
      selectedPerks: number[];
      championId: number;
      role?: number;
    }) => Promise<void>;
    onExport: (runesetId: number) => Promise<void>;
    onImport?: (pageId: number) => Promise<void>;
    onSelectChampion?: (champion: ChampionSummary | null) => void;
  } = $props();

  // Mode state: Default is View Mode (isEditing = false)
  let isEditing = $state<boolean>(false);
  let isChampionModalOpen = $state<boolean>(false);
  let allChampions = $state<ChampionSummary[]>([]);
  let runeChampion = $state<ChampionSummary | null>(null);

  // Builder filters
  let selectedRole = $state<number>(1); // Jungle default
  let backupRole = $state<number>(1);
  let selectedTiers = $state<number[]>([4, 5, 6, 7]); // Platinum+
  let pageName = $state<string>("Custom Rune Page");
  let saving = $state<boolean>(false);
  let exporting = $state<boolean>(false);
  let telemetryLoading = $state<boolean>(false);

  // Telemetry data
  let overview = $state<RuneBuilderOverview | null>(null);
  let keystones = $state<EnrichedKeystoneOption[]>([]);
  let selectedKeystoneId = $state<number | null>(null);
  let selectedPrimaryStyleId = $state<number>(8000);
  let selectedSubStyleId = $state<number>(8100);
  let primaryTree = $state<TreePerksStats | null>(null);
  let secondaryCandidates = $state<SecondaryTreeOption[]>([]);
  let secondaryTree = $state<TreePerksStats | null>(null);
  let statShards = $state<StatShardsStats>({ offense: [], flex: [], defense: [] });

  // 9 Selected Perk IDs
  let selectedPerkIds = $state<number[]>([]);

  // Backup for cancel
  let backupPerkIds = $state<number[]>([]);
  let backupName = $state<string>("");

  let lastLoadedTreeKey = $state<string>("");

  /**
   * Loads telemetry data for current champion and filters.
   */
  async function loadBuilderData(
    overrideKeystone?: number,
    overridePrimaryStyle?: number,
    overrideSubStyle?: number,
    overrideRole?: number,
  ) {
    try {
      telemetryLoading = true;
      const champId = runeChampion && runeChampion.id > 0 ? runeChampion.id : (selectedChampion && selectedChampion.id > 0 ? selectedChampion.id : 64);
      const primStyle = overridePrimaryStyle ?? selectedPrimaryStyleId;
      const subStyle = overrideSubStyle ?? selectedSubStyleId;
      const roleToUse = overrideRole !== undefined ? overrideRole : selectedRole;

      const query: BuilderQuery = {
        champion_id: champId,
        role: roleToUse,
        league_tiers: selectedTiers,
        keystone_id: overrideKeystone ?? selectedKeystoneId,
        primary_style_id: primStyle,
        sub_style_id: subStyle,
      };

      const res = await getBuilderOverview(query);
      overview = res;
      keystones = res.keystones || [];
      selectedKeystoneId = overrideKeystone ?? res.selected_keystone_id;
      selectedPrimaryStyleId = primStyle;
      selectedSubStyleId = subStyle;
      primaryTree = res.primary_tree;
      secondaryCandidates = res.secondary_tree_candidates || [];
      secondaryTree = res.secondary_tree;
      statShards = res.stat_shards || { offense: [], flex: [], defense: [] };

      if (selectedPerkIds.length < 9 && res.recommended_perk_ids) {
        selectedPerkIds = [...res.recommended_perk_ids];
      }
    } catch (err) {
      console.error("[Builder Telemetry Error]", err);
    } finally {
      telemetryLoading = false;
    }
  }

  // Reactively synchronize when external tree prop changes
  $effect(() => {
    if (tree) {
      const currentKey = `${tree.runeset_id || tree.page_id || ""}_${tree.page_name}`;
      if (currentKey !== lastLoadedTreeKey) {
        lastLoadedTreeKey = currentKey;
        pageName = tree.page_name || "Custom Rune Page";
        selectedPerkIds = tree.selected_perk_ids || [];
        backupPerkIds = [...selectedPerkIds];
        backupName = pageName;
        isEditing = false; // Stay in View Mode on page switch

        if (tree.champion_id && tree.champion_id > 0) {
          const found = allChampions.find((c) => c.id === tree.champion_id);
          runeChampion = found || null;
        } else {
          runeChampion = null;
        }

        if (tree.primary_style_id) selectedPrimaryStyleId = tree.primary_style_id;
        if (tree.sub_style_id) selectedSubStyleId = tree.sub_style_id;
        if (selectedPerkIds.length > 0) selectedKeystoneId = selectedPerkIds[0];

        const treeRole = tree.role !== undefined && tree.role !== null ? tree.role : 1;
        selectedRole = treeRole;
        backupRole = treeRole;

        loadBuilderData(selectedPerkIds[0], selectedPrimaryStyleId, selectedSubStyleId, treeRole);
      }
    }
  });

  onMount(async () => {
    try {
      allChampions = await getChampions();
      if (tree && tree.champion_id && tree.champion_id > 0) {
        const found = allChampions.find((c) => c.id === tree.champion_id);
        if (found) runeChampion = found;
      }
    } catch (err) {
      console.error("[RuneCanvas Champion Load Error]", err);
    }
    await loadBuilderData();
  });

  // Handler: Select primary style
  async function handleSelectPrimaryStyle(styleId: number) {
    if (selectedPrimaryStyleId === styleId) return;
    selectedPrimaryStyleId = styleId;

    // Default secondary tree to something different
    const candidateSub = [8000, 8100, 8200, 8300, 8400].find((s) => s !== styleId) || 8100;
    selectedSubStyleId = candidateSub;

    // Reset keystone & tree perks to first keystone
    await loadBuilderData(undefined, styleId, candidateSub);
  }

  // Handler: Select keystone
  async function handleSelectKeystone(keystoneId: number) {
    selectedKeystoneId = keystoneId;
    if (selectedPerkIds.length > 0) {
      selectedPerkIds[0] = keystoneId;
    } else {
      selectedPerkIds = [keystoneId];
    }
    await loadBuilderData(keystoneId);
  }

  // Handler: Select primary tier perk (indices 1, 2, 3)
  function handleSelectPrimaryPerk(perkId: number, rowIndex: number) {
    while (selectedPerkIds.length < 9) selectedPerkIds.push(0);
    selectedPerkIds[rowIndex + 1] = perkId;
  }

  // Handler: Select secondary tree tab
  async function handleSelectSubStyle(styleId: number) {
    selectedSubStyleId = styleId;
    try {
      const champId = selectedChampion && selectedChampion.id > 0 ? selectedChampion.id : 64;
      const query: BuilderQuery = {
        champion_id: champId,
        role: selectedRole,
        league_tiers: selectedTiers,
        keystone_id: selectedKeystoneId,
        primary_style_id: selectedPrimaryStyleId,
        sub_style_id: styleId,
      };
      secondaryTree = await getBuilderTreeRunes(selectedPrimaryStyleId, styleId, selectedKeystoneId, query);
    } catch (err) {
      console.error("[SubStyle Switch Error]", err);
    }
  }

  // Handler: Select secondary tier perk (slots 4, 5) ensuring ONLY 1 perk per row
  function handleSelectSecondaryPerk(perkId: number, rowIndex: number) {
    while (selectedPerkIds.length < 9) selectedPerkIds.push(0);

    const s4 = selectedPerkIds[4];
    const s5 = selectedPerkIds[5];

    // If this exact perk is already selected, do nothing
    if (s4 === perkId || s5 === perkId) return;

    // Find which row a perk belongs to in the current secondary tree
    const getRowForPerk = (id: number): number => {
      if (!secondaryTree || !id) return -1;
      return secondaryTree.rows.findIndex((r) => r.options.some((opt) => opt.id === id));
    };

    const row4 = getRowForPerk(s4);
    const row5 = getRowForPerk(s5);

    if (row4 === rowIndex) {
      selectedPerkIds[4] = perkId;
    } else if (row5 === rowIndex) {
      selectedPerkIds[5] = perkId;
    } else if (s4 === 0) {
      selectedPerkIds[4] = perkId;
    } else if (s5 === 0) {
      selectedPerkIds[5] = perkId;
    } else {
      selectedPerkIds[4] = s5;
      selectedPerkIds[5] = perkId;
    }
  }

  // Handler: Select stat shard (rows 0, 1, 2 -> indices 6, 7, 8)
  function handleSelectShard(shardId: number, rowIndex: number) {
    while (selectedPerkIds.length < 9) selectedPerkIds.push(0);
    selectedPerkIds[6 + rowIndex] = shardId;
  }

  // Handler: Role switcher
  async function handleSelectRole(role: number) {
    selectedRole = role;
    await loadBuilderData(selectedKeystoneId ?? undefined, selectedPrimaryStyleId, selectedSubStyleId, role);
  }

  // Handler: Rank tier toggle
  async function handleToggleTier(tier: number) {
    if (selectedTiers.includes(tier)) {
      if (selectedTiers.length > 1) {
        selectedTiers = selectedTiers.filter((t) => t !== tier);
      }
    } else {
      selectedTiers = [...selectedTiers, tier].sort((a, b) => a - b);
    }
    await loadBuilderData();
  }

  // Handler: Cancel Edit
  function handleCancelEdit() {
    selectedPerkIds = [...backupPerkIds];
    pageName = backupName;
    selectedRole = backupRole;
    isEditing = false;
    loadBuilderData(selectedPerkIds[0], selectedPrimaryStyleId, selectedSubStyleId, backupRole);
  }

  // Handler: Save
  async function handleSave() {
    try {
      saving = true;
      const champId = runeChampion && runeChampion.id > 0 ? runeChampion.id : 0;
      const defaultName = runeChampion ? `${runeChampion.name} Custom` : "Custom Rune Page";
      await onSave({
        name: pageName.trim() || defaultName,
        primaryStyleId: selectedPrimaryStyleId,
        subStyleId: selectedSubStyleId,
        selectedPerks: selectedPerkIds,
        championId: champId,
        role: selectedRole,
      });
      backupPerkIds = [...selectedPerkIds];
      backupName = pageName;
      backupRole = selectedRole;
      isEditing = false;
    } finally {
      saving = false;
    }
  }

  // Handler: Export
  async function handleExport() {
    if (tree?.runeset_id) {
      try {
        exporting = true;
        await onExport(tree.runeset_id);
      } finally {
        exporting = false;
      }
    } else {
      await handleSave();
    }
  }
</script>

<main class="flex-1 flex flex-col bg-gray-950/40 p-5 overflow-y-auto h-full relative select-none scrollbar-thin">
  <div class="w-full max-w-6xl mx-auto flex flex-col gap-4">
    <!-- Header: Mode Switcher, Page Title, Action Buttons & Filter Tray -->
    <RuneCanvasHeader
      {tree}
      {runeChampion}
      bind:pageName
      {isEditing}
      {saving}
      {exporting}
      {selectedRole}
      {selectedTiers}
      onOpenChampionModal={() => (isChampionModalOpen = true)}
      onEdit={() => (isEditing = true)}
      onCancel={handleCancelEdit}
      onSave={handleSave}
      onExport={handleExport}
      onSelectRole={handleSelectRole}
      onToggleTier={handleToggleTier}
    />

    <!-- Telemetry Skeleton / Loading -->
    {#if (telemetryLoading || loading) && !overview}
      <RuneLoadingSkeleton />
    {:else}
      <!-- Two Main Cards Layout: Left (Primary Path) and Right (Secondary Path & Shards) -->
      <div class="grid grid-cols-1 lg:grid-cols-2 gap-4 items-start">
        <!-- LEFT CARD: Primary Tree Selector + Keystones + 3 Tiers -->
        <PrimaryPathCard
          primaryStyleId={selectedPrimaryStyleId}
          {keystones}
          {selectedKeystoneId}
          tree={primaryTree}
          {selectedPerkIds}
          {isEditing}
          onSelectPrimaryStyle={handleSelectPrimaryStyle}
          onSelectKeystone={handleSelectKeystone}
          onSelectPerk={handleSelectPrimaryPerk}
        />

        <!-- RIGHT CARD: Secondary Tree Selector + 3 Tiers + Stat Shards -->
        <SecondaryPathCard
          primaryStyleId={selectedPrimaryStyleId}
          secondaryStyleId={selectedSubStyleId}
          candidates={secondaryCandidates}
          tree={secondaryTree}
          {statShards}
          {selectedPerkIds}
          {isEditing}
          onSelectSubStyle={handleSelectSubStyle}
          onSelectSecondaryPerk={handleSelectSecondaryPerk}
          onSelectShard={handleSelectShard}
        />
      </div>
    {/if}
  </div>

  <!-- Flowbite Champion Select Modal -->
  <ChampionSelectModal
    isOpen={isChampionModalOpen}
    title="Assign Champion to Rune Page"
    currentChampionId={runeChampion?.id}
    onSelect={(champ) => {
      runeChampion = champ;
      onSelectChampion?.(champ);
      loadBuilderData();
    }}
    onClose={() => (isChampionModalOpen = false)}
  />
</main>
