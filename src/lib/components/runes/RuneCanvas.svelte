<script lang="ts">
  import { onMount } from "svelte";
  import {
    AnimatePresence,
    MotionDiv,
    type MotionTransition,
  } from "@humanspeak/svelte-motion";
  import PrimaryPathCard from "./PrimaryPathCard.svelte";
  import SecondaryPathCard from "./SecondaryPathCard.svelte";
  import ChampionSelectModal from "../ChampionSelectModal.svelte";
  import Divider from "../Divider.svelte";
  import {
    getBuilderOverview,
    getBuilderTreeRunes,
  } from "../../api/rune_builder";
  import { getChampions } from "../../api/champions";

  import type { ChampionSummary } from "../../types/champions";
  import { ALL_CHAMPIONS_ICON_URL } from "../../types/champions";

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
      const champId = runeChampion && runeChampion.id > 0 ? runeChampion.id : (selectedChampion && selectedChampion.id > 0 ? selectedChampion.id : 64);
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

        if (tree.champion_id && tree.champion_id > 0) {
          const found = allChampions.find((c) => c.id === tree.champion_id);
          runeChampion = found || null;
        } else {
          runeChampion = null;
        }

        if (tree.primary_style_id) selectedPrimaryStyleId = tree.primary_style_id;
        if (tree.sub_style_id) selectedSubStyleId = tree.sub_style_id;
        if (selectedPerkIds.length > 0) selectedKeystoneId = selectedPerkIds[0];

        loadBuilderData(selectedPerkIds[0], selectedPrimaryStyleId, selectedSubStyleId);
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
      // Slot 4 already had a perk from this row: replace it
      selectedPerkIds[4] = perkId;
    } else if (row5 === rowIndex) {
      // Slot 5 already had a perk from this row: replace it
      selectedPerkIds[5] = perkId;
    } else if (s4 === 0) {
      // Slot 4 was empty: assign here
      selectedPerkIds[4] = perkId;
    } else if (s5 === 0) {
      // Slot 5 was empty: assign here
      selectedPerkIds[5] = perkId;
    } else {
      // Both slots 4 & 5 were from different rows: FIFO shift slot 4 -> slot 5
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
      const champId = runeChampion && runeChampion.id > 0 ? runeChampion.id : 0;
      const defaultName = runeChampion ? `${runeChampion.name} Custom` : "Custom Rune Page";
      await onSave({
        name: pageName.trim() || defaultName,
        primaryStyleId: selectedPrimaryStyleId,
        subStyleId: selectedSubStyleId,
        selectedPerks: selectedPerkIds,
        championId: champId,
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
  // Filter tray slide and fade transition for role/rank selectors
  const filterTransition: MotionTransition = {
    height: { duration: 0.5, ease: [0.16, 1, 0.3, 1] },
    opacity: { duration: 0.3, ease: "easeInOut" },
  };
</script>

<main class="flex-1 flex flex-col bg-gray-900 p-5 overflow-y-auto h-full relative select-none">
  <div class="w-full max-w-6xl mx-auto flex flex-col gap-4">
    <!-- Header: Mode Switcher, Page Title & Top Actions -->
    <div class="flex flex-col rounded-2xl border border-gray-800 bg-gray-950/90 p-4 shadow-md">
      <div class="flex flex-wrap items-center justify-between gap-3">
        <!-- Page Title & Champion Avatar / Asterisk -->
        <div class="flex items-center gap-2.5 flex-1 min-w-[240px]">
          <!-- Indicator Dot -->
          <span
            class="flex h-3 w-3 rounded-full {tree?.page_id ? 'bg-blue-500 shadow-blue-500' : 'bg-purple-500 shadow-purple-500'} shadow-sm shrink-0"
          ></span>

          <!-- Champion Icon with Clear Hover/Edit State in Edit Mode -->
          <button
            type="button"
            onclick={() => isEditing && (isChampionModalOpen = true)}
            disabled={!isEditing}
            title={isEditing ? `Champion: ${runeChampion?.name || 'All Champions'} (Click to change)` : (runeChampion?.name || "All Champions")}
            class={isEditing
              ? "group relative flex h-11 w-11 shrink-0 items-center justify-center overflow-hidden rounded-full border-2 border-purple-500 bg-gray-950 p-0 shadow-md shadow-purple-950/50 hover:border-purple-400 hover:scale-105 cursor-pointer transition-all"
              : "relative flex h-11 w-11 shrink-0 items-center justify-center overflow-hidden rounded-full border border-gray-700/80 bg-gray-950 p-0 shadow-md cursor-default"}
          >
            {#if runeChampion && runeChampion.id > 0 && runeChampion.square_icon_url}
              <img
                src={runeChampion.square_icon_url}
                alt={runeChampion.name}
                class="absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 h-[calc(100%+5px)] w-[calc(100%+5px)] object-cover rounded-full transition-all group-hover:brightness-50"
              />
            {:else}
              <img
                src={ALL_CHAMPIONS_ICON_URL}
                alt="All Champions"
                class="opacity-80 w-[70%] h-[70%] object-contain transition-all group-hover:brightness-50"
              />
            {/if}

            {#if isEditing}
              <!-- Hover Edit Badge Overlay -->
              <div
                class="absolute inset-0 bg-black/60 rounded-full flex flex-col items-center justify-center opacity-0 group-hover:opacity-100 transition-opacity z-10"
              >
                <span class="icon-[heroicons--pencil] h-3.5 w-3.5 text-purple-300"></span>
                <span class="text-[7px] font-black tracking-wider text-purple-200 uppercase">EDIT</span>
              </div>
            {/if}
          </button>

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
            <!-- Cancel Button (Dark / Solid Grey, Fixed 5rem / w-20) -->
            <button
              type="button"
              onclick={handleCancelEdit}
              class="h-8 inline-flex w-20 items-center justify-center gap-2 rounded-xl text-white bg-gray-800 hover:bg-gray-700 focus:ring-4 focus:ring-gray-700 box-border border border-transparent shadow-xs px-3.5 py-1.5 text-xs font-semibold focus:outline-none active:scale-95 cursor-pointer transition-all"
            >
              <span>Cancel</span>
            </button>

            <!-- Save Changes Button (Purple Action, Fixed 8rem / w-32) -->
            <button
              type="button"
              disabled={saving}
              onclick={handleSave}
              class="h-8 inline-flex w-32 items-center justify-center gap-2 rounded-xl bg-purple-600 px-3.5 py-1.5 text-xs font-semibold text-white shadow-md shadow-purple-950/50 transition-all hover:bg-purple-500 focus:outline-none focus:ring-4 focus:ring-purple-800 active:scale-95 cursor-pointer"
            >
              <span class="icon-[flowbite--floppy-disk-solid] h-4.5 w-4.5"></span>
              <span>{saving ? "Saving..." : "Save changes"}</span>
            </button>
          {:else}
            <!-- View Mode: Edit Button (Dark / Solid Grey, Fixed 5rem / w-20) -->
            <button
              title="Edit rune page"
              type="button"
              onclick={() => (isEditing = true)}
              class="h-8 inline-flex w-20 items-center justify-center gap-2 rounded-xl text-white bg-gray-800 hover:bg-gray-700 focus:ring-4 focus:ring-gray-700 box-border border border-transparent shadow-xs px-3.5 py-1.5 text-xs font-semibold focus:outline-none active:scale-95 cursor-pointer transition-all"
            >
              <span class="icon-[heroicons--pencil-square-16-solid] h-4.5 w-4.5"></span>
              <span>Edit</span>
            </button>

            <!-- View Mode: Send to Client Button (Gradient Duotone Purple to Blue, Fixed 8rem / w-32) -->
            <button
              type="button"
              disabled={exporting}
              onclick={handleExport}
              class="h-8 inline-flex w-34 items-center justify-center gap-2 rounded-xl text-white bg-gradient-to-br from-purple-600 to-blue-500 hover:bg-gradient-to-bl focus:ring-4 focus:ring-blue-300 dark:focus:ring-blue-800 shadow-md shadow-purple-950/40 px-3.5 py-1.5 text-xs font-semibold focus:outline-none active:scale-95 cursor-pointer transition-all"
            >
              <span class="icon-[heroicons--bolt-16-solid] h-4.5 w-4.5"></span>
              <span>{exporting ? "Sending..." : "Send to Client"}</span>
            </button>
          {/if}
        </div>
      </div>

      <!-- Filters (Role & Rank Tiers) Animated Expansion and Collapse with Owned Real-Node Exit -->
      <AnimatePresence present={isEditing}>
        {#snippet child()}
          <MotionDiv
            key="rune-filters-tray"
            initial={{ opacity: 0, height: 0 }}
            animate={{ opacity: 1, height: 55 }}
            exit={{ opacity: 0, height: 0 }}
            transition={filterTransition}
            class="overflow-hidden"
          >
            <!-- Separate Animated Divider Component -->
            <Divider transition={filterTransition} />

            <!-- Filter Controls Row -->
            <div class="flex flex-wrap items-center justify-between gap-3">
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
                        <span class="icon-[heroicons--check] h-2.5 w-2.5 text-purple-400"></span>
                      {/if}
                      <span>{tier.label}</span>
                    </button>
                  {/each}
                </div>
              </div>
            </div>
          </MotionDiv>
        {/snippet}
      </AnimatePresence>
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
