<script lang="ts">
  import { onMount } from "svelte";
  import PrimaryPathCard from "./PrimaryPathCard.svelte";
  import SecondaryPathCard from "./SecondaryPathCard.svelte";
  import {
    getBuilderOverview,
    getBuilderTreeRunes,
  } from "../../api/rune_builder";
  import {
    detectChampionFromName,
  } from "../../api/champions";

  import type { ChampionSummary } from "../../types/champions";
  import { ALL_CHAMPIONS_OPTION, ALL_CHAMPIONS_ICON_URL } from "../../types/champions";

  import type {
    BuilderQuery,
    EnrichedKeystoneOption,
    RuneBuilderOverview,
    SecondaryTreeOption,
    StatShardsStats,
    TreePerksStats,
  } from "../../types/rune_builder";
  import type { RuneTreePayload } from "../../types/runes";
  import { LEAGUE_ROLES_MAP } from "../../types/rune_builder";

  let {
    tree = null,
    loading = false,
    onSave,
    onExport,
    onImport,
    championFilter,
  }: {
    tree: RuneTreePayload | null;
    loading: boolean;
    onSave: (payload: {
      name: string;
      primaryStyleId: number;
      subStyleId: number;
      selectedPerks: number[];
      championId: number;
    }) => Promise<void>;
    onExport: (runesetId: number) => Promise<void>;
    onImport?: (pageId: number) => Promise<void>;
    championFilter?: ChampionSummary | null;
  } = $props();

  // Mode state: Default is View Mode (isEditing = false)
  let isEditing = $state<boolean>(false);

  // Champions state
  let selectedChampion = $state<ChampionSummary>(ALL_CHAMPIONS_OPTION);



  // Builder filters
  let selectedRole = $state<number>(1); // Jungle default
  let selectedTiers = $state<number[]>([4, 5, 6, 7]); // Platinum+
  let pageName = $state<string>("Custom Rune Page");
  let saving = $state<boolean>(false);
  let exporting = $state<boolean>(false);
  let importing = $state<boolean>(false);
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

  const roleEntries = Object.entries(LEAGUE_ROLES_MAP).map(([key, val]) => ({
    id: parseInt(key, 10),
    label: val.label,
  }));

  const tierEntries: { id: number; label: string }[] = [
    { id: 3, label: "Gold" },
    { id: 4, label: "Platinum" },
    { id: 5, label: "Emerald" },
    { id: 6, label: "Diamond" },
    { id: 7, label: "Master+" },
  ];

  /**
   * Loads telemetry data for current champion and filters.
   */
  async function loadBuilderData(overrideKeystone?: number, overridePrimaryStyle?: number, overrideSubStyle?: number) {
    try {
      telemetryLoading = true;
      const champId = selectedChampion.id > 0 ? selectedChampion.id : 64;
      const primStyle = overridePrimaryStyle ?? selectedPrimaryStyleId;
      const subStyle = overrideSubStyle ?? selectedSubStyleId;

      const query: BuilderQuery = {
        champion_id: champId,
        role: selectedRole,
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

        if (tree.primary_style_id) selectedPrimaryStyleId = tree.primary_style_id;
        if (tree.sub_style_id) selectedSubStyleId = tree.sub_style_id;
        if (selectedPerkIds.length > 0) selectedKeystoneId = selectedPerkIds[0];

        detectChampionFromName(tree.page_name).then((detected) => {
          selectedChampion = detected || ALL_CHAMPIONS_OPTION;
          loadBuilderData(selectedPerkIds[0], selectedPrimaryStyleId, selectedSubStyleId);
        });
      }
    }
  });

  let lastChampionFilterId = $state<number | undefined>(undefined);

  $effect(() => {
    if (championFilter !== undefined && championFilter?.id !== lastChampionFilterId) {
      lastChampionFilterId = championFilter?.id;
      selectedChampion = championFilter || ALL_CHAMPIONS_OPTION;
      loadBuilderData();
    }
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
      const champId = selectedChampion.id > 0 ? selectedChampion.id : 64;
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

  // Handler: Select secondary tier perk (slots 4, 5)
  function handleSelectSecondaryPerk(perkId: number) {
    while (selectedPerkIds.length < 9) selectedPerkIds.push(0);
    if (selectedPerkIds[4] === perkId || selectedPerkIds[5] === perkId) return;
    selectedPerkIds[4] = selectedPerkIds[5];
    selectedPerkIds[5] = perkId;
  }

  // Handler: Select stat shard (rows 0, 1, 2 -> indices 6, 7, 8)
  function handleSelectShard(shardId: number, rowIndex: number) {
    while (selectedPerkIds.length < 9) selectedPerkIds.push(0);
    selectedPerkIds[6 + rowIndex] = shardId;
  }

  // Handler: Role switcher
  async function handleSelectRole(role: number) {
    selectedRole = role;
    await loadBuilderData();
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

  // Handler: Apply recommended
  function handleApplyRecommended() {
    if (overview?.recommended_perk_ids && overview.recommended_perk_ids.length >= 9) {
      selectedPerkIds = [...overview.recommended_perk_ids];
      if (overview.selected_keystone_id) {
        selectedKeystoneId = overview.selected_keystone_id;
      }
    }
  }

  // Handler: Cancel Edit
  function handleCancelEdit() {
    selectedPerkIds = [...backupPerkIds];
    pageName = backupName;
    isEditing = false;
  }

  // Handler: Save
  async function handleSave() {
    try {
      saving = true;
      await onSave({
        name: pageName.trim() || `${selectedChampion.name} Custom`,
        primaryStyleId: selectedPrimaryStyleId,
        subStyleId: selectedSubStyleId,
        selectedPerks: selectedPerkIds,
        championId: selectedChampion.id,
      });
      backupPerkIds = [...selectedPerkIds];
      backupName = pageName;
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

  // Handler: Import
  async function handleImport() {
    if (tree?.page_id && onImport) {
      try {
        importing = true;
        await onImport(tree.page_id);
      } finally {
        importing = false;
      }
    }
  }

  onMount(async () => {
    await loadBuilderData();
  });

</script>

<main class="flex-1 flex flex-col bg-gray-900 p-5 overflow-y-auto h-full relative select-none">
  <div class="w-full max-w-6xl mx-auto flex flex-col gap-4">
    <!-- Header: Mode Switcher, Page Title & Top Actions -->
    <div class="flex flex-col gap-3 rounded-2xl border border-gray-800 bg-gray-950/90 p-4 shadow-md">
      <div class="flex flex-wrap items-center justify-between gap-3">
        <!-- Page Title & Champion Avatar / Asterisk -->
        <div class="flex items-center gap-2.5 flex-1 min-w-[240px]">
          <!-- Indicator Dot -->
          <span
            class="flex h-3 w-3 rounded-full {tree?.page_id ? 'bg-blue-500 shadow-blue-500' : 'bg-purple-500 shadow-purple-500'} shadow-sm shrink-0"
          ></span>

          <!-- Champion Icon or Autofill Icon for all -->
          <div
            class="relative flex h-11 w-11 shrink-0 items-center justify-center overflow-hidden rounded-full border border-gray-700/80 bg-gray-950 p-0 shadow-md"
          >
            {#if selectedChampion.id > 0 && selectedChampion.square_icon_url}
              <img
                src={selectedChampion.square_icon_url}
                alt={selectedChampion.name}
                title={selectedChampion.name}
                class="absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 h-[calc(100%+5px)] w-[calc(100%+5px)] object-cover rounded-full"
              />
            {:else}
              <img
                src={ALL_CHAMPIONS_ICON_URL}
                alt="All Champions"
                title="All Champions"
                class="opacity-80 w-[70%] h-[70%] object-contain"
              />
            {/if}
          </div>



          <!-- Page Name Display (View Mode) or Input (Edit Mode) -->
          {#if isEditing}
            <input
              type="text"
              bind:value={pageName}
              placeholder="Rune page name..."
              class="w-full max-w-xs rounded-xl border border-gray-700 bg-gray-900 px-3 py-1.5 text-sm font-bold text-white placeholder:text-gray-500 focus:border-purple-500 focus:outline-none focus:ring-1 focus:ring-purple-500"
            />
          {:else}
            <div class="flex items-center gap-2">
              <h2 class="text-base font-extrabold text-white truncate max-w-md">
                {pageName || tree?.page_name || "Custom Rune Page"}
              </h2>
            </div>
          {/if}
        </div>

        <!-- Action Buttons -->
        <div class="flex items-center gap-2 shrink-0">
          {#if isEditing}
            <!-- Apply Recommended Button -->
            <button
              type="button"
              title="Auto-select highest winrate combination"
              onclick={handleApplyRecommended}
              class="inline-flex items-center gap-1.5 rounded-xl border border-blue-500/40 bg-blue-500/10 px-3 py-1.5 text-xs font-bold text-blue-300 transition-all hover:border-blue-500 hover:bg-blue-600 hover:text-white cursor-pointer"
            >
              <svg class="h-3.5 w-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 10V3L4 14h7v7l9-11h-7z"/>
              </svg>
              <span>Recommended</span>
            </button>

            <!-- Cancel Button -->
            <button
              type="button"
              onclick={handleCancelEdit}
              class="inline-flex items-center gap-1.5 rounded-xl border border-gray-700 bg-gray-800 px-3 py-1.5 text-xs font-bold text-gray-300 transition-all hover:bg-gray-700 hover:text-white cursor-pointer"
            >
              <span>Cancel</span>
            </button>

            <!-- Save Button -->
            <button
              type="button"
              disabled={saving}
              onclick={handleSave}
              class="inline-flex items-center gap-1.5 rounded-xl bg-purple-600 px-4 py-1.5 text-xs font-bold text-white shadow-md shadow-purple-950/50 transition-all hover:bg-purple-500 active:scale-95 cursor-pointer"
            >
              <svg class="h-3.5 w-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7"/>
              </svg>
              <span>{saving ? "Saving..." : "Save Page"}</span>
            </button>
          {:else}
            <!-- View Mode: Edit Button -->
            <button
              type="button"
              onclick={() => (isEditing = true)}
              class="inline-flex items-center gap-1.5 rounded-xl border border-purple-500/40 bg-purple-500/10 px-3.5 py-1.5 text-xs font-bold text-purple-300 transition-all hover:border-purple-500 hover:bg-purple-600 hover:text-white cursor-pointer"
            >
              <svg class="h-3.5 w-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M11 5H6a2 2 0 00-2 2v11a2 2 0 002 2h11a2 2 0 002-2v-5m-1.414-9.414a2 2 0 112.828 2.828L11.828 15H9v-2.828l8.586-8.586z"/>
              </svg>
              <span>Edit Page</span>
            </button>

            <!-- View Mode: Import Button (for Client Pages) -->
            {#if tree?.page_id && onImport}
              <button
                type="button"
                disabled={importing}
                onclick={handleImport}
                class="inline-flex items-center gap-1.5 rounded-xl border border-blue-500/40 bg-blue-500/10 px-3.5 py-1.5 text-xs font-bold text-blue-300 transition-all hover:border-blue-500 hover:bg-blue-600 hover:text-white cursor-pointer"
              >
                <svg class="h-3.5 w-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-4l-4 4m0 0l-4-4m4 4V4"/>
                </svg>
                <span>{importing ? "Importing..." : "Import"}</span>
              </button>
            {/if}


            <!-- View Mode: Export Button -->
            <button
              type="button"
              disabled={exporting}
              onclick={handleExport}
              class="inline-flex items-center gap-1.5 rounded-xl bg-purple-600 px-4 py-1.5 text-xs font-bold text-white shadow-md shadow-purple-950/50 transition-all hover:bg-purple-500 active:scale-95 cursor-pointer"
            >
              <svg class="h-3.5 w-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-8l-4-4m0 0L8 8m4-4v12"/>
              </svg>
              <span>{exporting ? "Exporting..." : "Export to Game"}</span>
            </button>
          {/if}
        </div>
      </div>

      <!-- Filters (Role & Rank Tiers) -->
      {#if isEditing}
        <div class="flex flex-wrap items-center justify-between gap-3 border-t border-gray-800/80 pt-3">
          <!-- Role Tabs -->
          <div class="flex items-center gap-1">
            <span class="text-[10px] font-bold uppercase tracking-wider text-gray-400 mr-1">Role:</span>
            <div class="inline-flex rounded-xl bg-gray-900 p-0.5 border border-gray-800 shadow-sm">
              {#each roleEntries as role (role.id)}
                <button
                  type="button"
                  onclick={() => handleSelectRole(role.id)}
                  class={selectedRole === role.id
                    ? "rounded-lg bg-purple-600 px-2.5 py-1 text-xs font-bold text-white shadow-sm cursor-pointer"
                    : "rounded-lg px-2.5 py-1 text-xs font-semibold text-gray-400 hover:text-white cursor-pointer transition-colors"}
                >
                  {role.label}
                </button>
              {/each}
            </div>
          </div>

          <!-- Rank Tier Filter Badges -->
          <div class="flex items-center gap-1">
            <span class="text-[10px] font-bold uppercase tracking-wider text-gray-400 mr-1">Ranks:</span>
            <div class="flex flex-wrap items-center gap-1">
              {#each tierEntries as tier (tier.id)}
                {@const isChecked = selectedTiers.includes(tier.id)}
                <button
                  type="button"
                  onclick={() => handleToggleTier(tier.id)}
                  class={isChecked
                    ? "inline-flex items-center gap-1 rounded-full border border-purple-500/50 bg-purple-500/20 px-2 py-0.5 text-[10px] font-bold text-purple-300 shadow-sm cursor-pointer"
                    : "inline-flex items-center gap-1 rounded-full border border-gray-800 bg-gray-900/60 px-2 py-0.5 text-[10px] font-medium text-gray-400 hover:border-gray-700 hover:text-gray-300 cursor-pointer"}
                >
                  {#if isChecked}
                    <svg class="h-2.5 w-2.5 text-purple-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="3" d="M5 13l4 4L19 7"/>
                    </svg>
                  {/if}
                  <span>{tier.label}</span>
                </button>
              {/each}
            </div>
          </div>
        </div>
      {/if}
    </div>

    <!-- Telemetry Skeleton / Loading -->
    {#if (telemetryLoading || loading) && !overview}
      <div class="rounded-2xl border border-gray-800 bg-gray-950/60 p-12 text-center animate-pulse">
        <div class="mx-auto flex h-10 w-10 items-center justify-center rounded-xl bg-gray-800 text-purple-400 mb-3">
          <svg class="h-6 w-6 animate-spin" fill="none" viewBox="0 0 24 24">
            <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
            <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z"></path>
          </svg>
        </div>
        <h4 class="text-sm font-bold text-gray-200">Fetching Live Telemetry & Winrates...</h4>
        <p class="text-xs text-gray-500 mt-1">Analyzing millions of match combinations</p>
      </div>
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
</main>
