<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import TitleBar from "./lib/components/TitleBar.svelte";
  import TopNavBar from "./lib/components/navigation/TopNavBar.svelte";
  import RunesSidebar from "./lib/components/sidebar/RunesSidebar.svelte";
  import RuneCanvas from "./lib/components/runes/RuneCanvas.svelte";
  import MatchHistoryView from "./lib/components/history/MatchHistoryView.svelte";
  import ProfileStatisticsView from "./lib/components/history/ProfileStatisticsView.svelte";
  import BuildStatisticsView from "./lib/components/history/BuildStatisticsView.svelte";
  import MayhemChampSelectView from "./lib/components/history/MayhemChampSelectView.svelte";
  import {
    deleteRuneset,
    exportSavedRunePage,
    getClientRuneTree,
    getCurrentRuneTree,
    getRunesOverview,
    getSavedRuneTree,
    getSelectedRuneKey,
    importClientPage,
    saveRuneTree,
    setSelectedRuneKey,
    updateRunesetChampion,
  } from "./lib/api/runes";
  import {
    getChampions,
    getSelectedChampionId,
    setSelectedChampionId,
    toggleFavoriteChampion,
  } from "./lib/api/champions";
  import type {
    ClientRunePageSummary,
    RuneSetSummary,
    RuneTreePayload,
    SaveRuneTreePayload,
  } from "./lib/types/runes";
  import type { ChampionSummary } from "./lib/types/champions";
  import { logger } from "./lib/utils/logger";
  import {
    getGlobalAramConfig,
    resetGlobalAramDatabase,
    setGlobalAramLimit,
    type AgentOperation,
    type GlobalAramPatchBreakdown,
  } from "./lib/api/agent";
  import {
    clearStatisticsCache,
    syncStatisticsCachePatch,
  } from "./lib/utils/statistics-data";

  let clientPages = $state<ClientRunePageSummary[]>([]);
  let savedRunesets = $state<RuneSetSummary[]>([]);
  let activeCompanionName = $state<string | null>(null);
  let selectedKey = $state<string>("");
  let selectedChampion = $state<ChampionSummary | null>(null);
  let allChampions = $state<ChampionSummary[]>([]);
  let activeTab = $state<"builds" | "mayhem" | "profile">("builds");
  let mayhemTab = $state<"statistics" | "builds">("statistics");
  let statisticsPerspective = $state<"champions" | "builds">("champions");
  let statisticsScope = $state<"personal" | "compare">("personal");
  let statisticsConfigOpen = $state(false);
  let globalLimit = $state(1000);
  let globalGames = $state(0);
  let globalCollectorState = $state("unknown");
  let globalQueuedJobs = $state(0);
  let globalCurrentPatch = $state<string | null>(null);
  let globalPatches = $state<GlobalAramPatchBreakdown[]>([]);
  let globalOutdatedGames = $state(0);
  let globalHasPatchMismatch = $state(false);
  let confirmingGlobalReset = $state(false);
  let resettingGlobalDb = $state(false);
  let statisticsRefreshKey = $state(0);
  let agentOperations = $state<AgentOperation[]>([]);
  let savingGlobalLimit = $state(false);
  let hideLowPickrate = $state(false);
  let crawlerRateText = $state("0.0 match/min");

  let currentTree = $state<RuneTreePayload | null>(null);
  let sidebarLoading = $state(true);
  let treeLoading = $state(true);
  let topNavBarRef = $state<TopNavBar | null>(null);
  let isSidebarOpen = $state<boolean>(true);

  onMount(() => {
    const savedTab = localStorage.getItem("lcu-active-tab");
    if (savedTab === "builds" || savedTab === "mayhem" || savedTab === "profile") {
      activeTab = savedTab;
    }
    const savedProfileTab = localStorage.getItem("lcu-profile-tab");
    if (savedProfileTab === "statistics") {
      // Migrate legacy profile statistics to Mayhem tab
      activeTab = "mayhem";
      mayhemTab = "statistics";
      localStorage.setItem("lcu-active-tab", "mayhem");
      localStorage.removeItem("lcu-profile-tab");
    }
    const savedMayhemTab = localStorage.getItem("lcu-mayhem-tab");
    if (savedMayhemTab === "statistics" || savedMayhemTab === "builds") {
      mayhemTab = savedMayhemTab;
    }
    const savedStatisticsPerspective = localStorage.getItem("lcu-statistics-perspective");
    if (savedStatisticsPerspective === "champions" || savedStatisticsPerspective === "builds") {
      statisticsPerspective = savedStatisticsPerspective;
    }
    const savedStatisticsScope = localStorage.getItem("lcu-statistics-scope");
    if (savedStatisticsScope === "personal" || savedStatisticsScope === "compare") {
      statisticsScope = savedStatisticsScope;
    }
    hideLowPickrate = localStorage.getItem("lcu-statistics-hide-low-pickrate") === "true";
    let pollTimer: ReturnType<typeof setTimeout> | null = null;
    let destroyed = false;

    const refreshGlobalStatus = async () => {
      if (destroyed) return;
      let hasActiveWork = false;
      try {
        const config = await getGlobalAramConfig();
        if (!statisticsConfigOpen) {
          globalLimit = config.limit;
        }
        globalGames = config.games;
        globalCollectorState = config.collector_state;
        globalQueuedJobs = config.queued_jobs;
        globalCurrentPatch = config.current_patch ?? null;
        globalPatches = config.patches ?? [];
        globalOutdatedGames = config.outdated_games ?? 0;
        globalHasPatchMismatch = Boolean(config.has_patch_mismatch);
        agentOperations = config.operations ?? [];
        crawlerRateText = config.crawler_rate_text ?? "0.0 match/min";
        if (syncStatisticsCachePatch(globalCurrentPatch)) {
          statisticsRefreshKey += 1;
        }

        hasActiveWork =
          agentOperations.length > 0 ||
          (globalCollectorState !== "live" &&
            globalCollectorState !== "sleeping" &&
            globalCollectorState !== "unknown" &&
            !globalCollectorState.includes("paused"));
      } catch (error) {
        logger.warn("frontend.app", "Global statistics config unavailable", error);
      }

      if (!destroyed) {
        // Poll quickly (1s) only when tasks/crawlers are running, otherwise chill at 6s
        const nextDelay = hasActiveWork ? 1000 : 6000;
        pollTimer = setTimeout(() => void refreshGlobalStatus(), nextDelay);
      }
    };

    void refreshGlobalStatus();
    return () => {
      destroyed = true;
      if (pollTimer) clearTimeout(pollTimer);
    };
  });

  function handleTabChange(tab: "builds" | "mayhem" | "profile") {
    activeTab = tab;
    localStorage.setItem("lcu-active-tab", tab);
  }

  function handleMayhemTabChange(tab: "statistics" | "builds") {
    mayhemTab = tab;
    localStorage.setItem("lcu-mayhem-tab", tab);
  }

  function handleStatisticsPerspectiveChange(perspective: "champions" | "builds") {
    statisticsPerspective = perspective;
    localStorage.setItem("lcu-statistics-perspective", perspective);
  }

  function handleStatisticsScopeChange(scope: "personal" | "compare") {
    statisticsScope = scope;
    localStorage.setItem("lcu-statistics-scope", scope);
  }

  async function saveGlobalStatisticsLimit() {
    savingGlobalLimit = true;
    try {
      const limitResult = await setGlobalAramLimit(
        Math.max(1, Math.min(100000, Math.floor(globalLimit)))
      );
      globalLimit = limitResult.limit;
      confirmingGlobalReset = false;
      statisticsConfigOpen = false;
    } catch (error) {
      logger.error("frontend.app", "Failed to save global statistics settings", error);
    } finally {
      savingGlobalLimit = false;
    }
  }

  async function handleResetGlobalDatabase() {
    resettingGlobalDb = true;
    try {
      const result = await resetGlobalAramDatabase();
      clearStatisticsCache();
      globalGames = result.games;
      globalQueuedJobs = result.queued_jobs;
      globalPatches = [];
      globalOutdatedGames = 0;
      globalHasPatchMismatch = false;
      globalCollectorState = `global-aram:0/${result.limit} (reset)`;
      crawlerRateText = "0.0 match/min";
      confirmingGlobalReset = false;
      statisticsRefreshKey += 1;
    } catch (error) {
      logger.error("frontend.app", "Failed to reset global ARAM database", error);
    } finally {
      resettingGlobalDb = false;
    }
  }

  function handleHideLowPickrateChange(event: Event) {
    const input = event.currentTarget as HTMLInputElement;
    hideLowPickrate = input.checked;
    localStorage.setItem("lcu-statistics-hide-low-pickrate", String(input.checked));
  }

  function toggleSidebar() {
    isSidebarOpen = !isSidebarOpen;
  }

  async function loadChampions() {
    try {
      allChampions = await getChampions();
      topNavBarRef?.refreshChampions();
    } catch (err) {
      logger.error("frontend.app", "Failed to load champions", err);
    }
  }

  async function restoreSelectedChampion() {
    try {
      const savedId = await getSelectedChampionId();
      if (savedId && savedId > 0) {
        const matched = allChampions.find((c) => c.id === savedId);
        if (matched) {
          selectedChampion = matched;
        }
      }
    } catch (err) {
      logger.error("frontend.app", "Failed to restore selected champion", err);
    }
  }

  async function handleSelectChampionFilter(champ: ChampionSummary | null) {
    selectedChampion = champ;
    try {
      await setSelectedChampionId(champ ? champ.id : null);
    } catch (err) {
      logger.error("frontend.app", "Failed to save selected champion", err);
    }
  }

  async function handleToggleFavorite(championId: number) {
    try {
      const updated = await toggleFavoriteChampion(championId);
      allChampions = allChampions.map((c) =>
        c.id === championId ? { ...c, is_favorite: updated } : c,
      );
      topNavBarRef?.refreshChampions();
    } catch (err) {
      logger.error("frontend.app", "Failed to toggle favorite champion", err);
    }
  }

  async function loadOverview(preserveKey?: string): Promise<string> {
    try {
      sidebarLoading = true;
      const overview = await getRunesOverview();
      clientPages = overview.client_pages || [];
      savedRunesets = overview.saved_runesets || [];
      activeCompanionName = overview.active_companion_name || null;

      let candidateKey = preserveKey || selectedKey;
      let keyToSelect = "";

      // Validate candidate key against loaded pages
      if (candidateKey) {
        if (
          candidateKey.startsWith("saved:") &&
          savedRunesets.some((s) => `saved:${s.id}` === candidateKey)
        ) {
          keyToSelect = candidateKey;
        } else if (
          candidateKey.startsWith("client:") &&
          clientPages.some((c) => `client:${c.id}` === candidateKey)
        ) {
          keyToSelect = candidateKey;
        }
      }

      // Fallback selection if no valid candidate key
      if (!keyToSelect) {
        if (savedRunesets.length > 0) {
          keyToSelect = `saved:${savedRunesets[0].id}`;
        } else if (clientPages.some((p) => p.is_active)) {
          const active = clientPages.find((p) => p.is_active)!;
          keyToSelect = `client:${active.id}`;
        } else if (clientPages.length > 0) {
          keyToSelect = `client:${clientPages[0].id}`;
        }
      }

      selectedKey = keyToSelect;
      return keyToSelect;
    } finally {
      sidebarLoading = false;
    }
  }

  async function loadTreeForKey(key: string) {
    try {
      treeLoading = true;
      selectedKey = key;
      if (key.startsWith("saved:")) {
        const id = parseInt(key.slice("saved:".length), 10);
        currentTree = await getSavedRuneTree(id);
      } else if (key.startsWith("client:")) {
        const id = parseInt(key.slice("client:".length), 10);
        currentTree = await getClientRuneTree(id);
      } else {
        currentTree = await getCurrentRuneTree();
      }
    } catch (err) {
      logger.error("frontend.app", "Failed to load rune tree", err);
    } finally {
      treeLoading = false;
    }
  }

  async function handleSelectPage(key: string) {
    try {
      await setSelectedRuneKey(key);
    } catch (err) {
      logger.error("frontend.app", "Failed to save selected rune key", err);
    }
    await loadTreeForKey(key);
  }

  async function handleImportPage(pageId: number) {
    try {
      const newId = await importClientPage(pageId, selectedChampion?.id);
      const newKey = `saved:${newId}`;
      await setSelectedRuneKey(newKey);
      await loadOverview(newKey);
      await loadTreeForKey(newKey);
      topNavBarRef?.refreshChampions();
    } catch (err) {
      logger.error("frontend.app", "Failed to import rune page", err);
    }
  }

  async function handleExportPage(runesetId: number) {
    try {
      await exportSavedRunePage(runesetId);
      await loadOverview();
      topNavBarRef?.refreshProfile();
    } catch (err) {
      logger.error("frontend.app", "Failed to export rune page", err);
    }
  }

  async function handleDeletePage(runesetId: number) {
    try {
      await deleteRuneset(runesetId);
      const nextKey = await loadOverview();
      if (nextKey) {
        await setSelectedRuneKey(nextKey);
        await loadTreeForKey(nextKey);
      } else {
        await setSelectedRuneKey(null);
        currentTree = null;
      }
      topNavBarRef?.refreshChampions();
    } catch (err) {
      logger.error("frontend.app", "Failed to delete rune page", err);
    }
  }

  async function handleUpdateRunesetChampion(
    runesetId: number,
    championId: number,
  ) {
    try {
      await updateRunesetChampion(runesetId, championId);
      await loadOverview();
      if (selectedKey === `saved:${runesetId}`) {
        await loadTreeForKey(`saved:${runesetId}`);
      }
      topNavBarRef?.refreshChampions();
    } catch (err) {
      logger.error("frontend.app", "Failed to update runeset champion", err);
    }
  }

  async function handleSaveTree(data: {
    name: string;
    primaryStyleId: number;
    subStyleId: number;
    selectedPerks: number[];
    championId: number;
    role?: number;
  }) {
    const payload: SaveRuneTreePayload = {
      runeset_id: currentTree?.runeset_id,
      page_id: currentTree?.page_id,
      page_name: data.name,
      primary_style_id: data.primaryStyleId,
      sub_style_id: data.subStyleId,
      selected_perks: data.selectedPerks,
      champion_id: data.championId,
      role: data.role,
    };

    const savedId = await saveRuneTree(payload);
    const savedKey = `saved:${savedId}`;
    await setSelectedRuneKey(savedKey);
    await loadOverview(savedKey);
    await loadTreeForKey(savedKey);
    topNavBarRef?.refreshChampions();
  }

  let unlistenRunes: UnlistenFn | null = null;
  let unlistenConnected: UnlistenFn | null = null;
  let unlistenDisconnected: UnlistenFn | null = null;

  onMount(async () => {
    await loadChampions();
    await restoreSelectedChampion();
    const persistedRuneKey = await getSelectedRuneKey();
    const key = await loadOverview(persistedRuneKey || undefined);
    if (key) {
      await loadTreeForKey(key);
    }

    try {
      unlistenRunes = await listen("lcu:runes-changed", async () => {
        await loadOverview(selectedKey);
      });
      unlistenConnected = await listen("lcu:connected", async () => {
        await loadOverview(selectedKey);
        await loadChampions();
        topNavBarRef?.refreshChampions();
        topNavBarRef?.refreshProfile();
      });
      unlistenDisconnected = await listen("lcu:disconnected", async () => {
        clientPages = [];
        activeCompanionName = null;
        await loadOverview(selectedKey);
      });
    } catch (err) {
      logger.error("frontend.app", "Failed to attach Tauri event listeners", err);
    }
  });

  onDestroy(() => {
    if (unlistenRunes) unlistenRunes();
    if (unlistenConnected) unlistenConnected();
    if (unlistenDisconnected) unlistenDisconnected();
  });
</script>

<div
  class="h-screen w-screen flex flex-col overflow-hidden bg-gray-900 text-gray-100 antialiased selection:bg-purple-600 selection:text-white"
>
  <!-- Draggable Custom Title Bar with App Branding & Window Controls -->
  <TitleBar {activeTab} onTabChange={handleTabChange} />

  <!-- Topside Header Navigation Bar (Sidebar Toggle + Profile + Champion Search) -->
  <TopNavBar
    bind:this={topNavBarRef}
    {isSidebarOpen}
    onToggleSidebar={toggleSidebar}
    {selectedChampion}
    {allChampions}
    activeRuneChampionId={currentTree?.champion_id || 0}
    onSelectChampionFilter={handleSelectChampionFilter}
    onToggleFavoriteChampion={handleToggleFavorite}
  />

  <!-- Main App Shell -->
  <div
    id="app-shell"
    class="flex min-h-0 flex-1 overflow-hidden bg-gray-900 text-gray-100 relative scrollbar-thumb-gray-800 scrollbar-track-gray-900 scrollbar-thin"
  >
    {#if activeTab === "builds"}
      <!-- Collapsible Sidebar Component -->
      <RunesSidebar
        bind:isOpen={isSidebarOpen}
        {clientPages}
        {savedRunesets}
        {activeCompanionName}
        {selectedKey}
        {selectedChampion}
        {allChampions}
        loading={sidebarLoading}
        onSelectPage={handleSelectPage}
        onImportPage={handleImportPage}
        onDeletePage={handleDeletePage}
        onChangeRunesetChampion={handleUpdateRunesetChampion}
      />

      <!-- Center Canvas: Active / Selected Rune Tree -->
      <RuneCanvas
        tree={currentTree}
        loading={treeLoading}
        {selectedChampion}
        onSave={handleSaveTree}
        onExport={handleExportPage}
        onImport={handleImportPage}
        onSelectChampion={handleSelectChampionFilter}
      />
    {:else if activeTab === "mayhem"}
      <div class="flex min-h-0 flex-1 overflow-hidden">
        <nav class="flex w-16 shrink-0 flex-col items-center gap-2 border-r border-gray-800 bg-gray-950 py-4">
          <button
            type="button"
            title="Statistics"
            aria-label="Statistics"
            onclick={() => handleMayhemTabChange("statistics")}
            class="flex h-11 w-11 items-center justify-center rounded-xl transition-colors {mayhemTab === 'statistics' ? 'bg-purple-600 text-white' : 'text-gray-500 hover:bg-gray-800 hover:text-gray-200'}"
          >
            <span class="icon-[heroicons--chart-bar] h-5 w-5"></span>
          </button>
          <button
            type="button"
            title="Builds & Champion Selection"
            aria-label="Builds & Champion Selection"
            onclick={() => handleMayhemTabChange("builds")}
            class="flex h-11 w-11 items-center justify-center rounded-xl transition-colors {mayhemTab === 'builds' ? 'bg-purple-600 text-white' : 'text-gray-500 hover:bg-gray-800 hover:text-gray-200'}"
          >
            <span class="icon-[heroicons--sparkles] h-5 w-5"></span>
          </button>
        </nav>
        <div class="relative min-h-0 min-w-0 flex-1 flex flex-col bg-gray-950">
          {#if mayhemTab === "builds"}
            <MayhemChampSelectView />
          {:else}
          <!-- Thin Crawler Topbar Loader when Crawler is Active (position absolute, right of sidebar) -->
          {#if globalCollectorState.startsWith("global-aram:") || globalCollectorState === "collecting" || agentOperations.some((op) => op.id === "global-database")}
            {@const crawlerOp = agentOperations.find((op) => op.id === "global-database")}
            {@const currentCount = crawlerOp?.current ?? globalGames}
            {@const totalLimit = crawlerOp?.total ?? globalLimit}
            {@const progressPct = Math.min(100, Math.max(0, Math.round((currentCount / Math.max(1, totalLimit)) * 100)))}
            <div class="absolute left-0 right-0 top-0 z-20 select-none pointer-events-none">
              <!-- 1. Pure, thin, vivid progress bar line (no text inside, strong color) -->
              <div class="h-[3px] w-full bg-cyan-950/60 overflow-hidden">
                <div
                  class="h-full bg-cyan-400 shadow-[0_0_8px_rgba(34,211,238,0.8)] transition-[width] duration-300 ease-out"
                  style={`width: ${progressPct}%`}
                ></div>
              </div>

              <!-- 2. Status row underneath the bar: left-aligned counter, percentage, and progress donut aligned with statistics header (px-4) -->
              <div class="flex items-center justify-start px-4 pt-2 pb-1 text-xs">
                <div class="flex items-center gap-2.5 py-1">
                  <!-- Progress donut filling with progress -->
                  <svg class="h-4 w-4 -rotate-90 shrink-0" viewBox="0 0 24 24">
                    <circle
                      class="text-cyan-950/80"
                      stroke="currentColor"
                      stroke-width="3"
                      fill="transparent"
                      cx="12"
                      cy="12"
                      r="9"
                    />
                    <circle
                      class="text-cyan-400 transition-[stroke-dashoffset] duration-300 ease-out"
                      stroke="currentColor"
                      stroke-width="3"
                      stroke-linecap="round"
                      fill="transparent"
                      cx="12"
                      cy="12"
                      r="9"
                      stroke-dasharray="56.548"
                      stroke-dashoffset={56.548 - (progressPct / 100) * 56.548}
                    />
                  </svg>
                  <span class="tabular-nums font-semibold tracking-wider text-cyan-200">
                    {currentCount.toLocaleString()} / {totalLimit.toLocaleString()}
                  </span>
                  <span class="text-[11px] font-medium text-cyan-400/90 tabular-nums">
                    ({progressPct}%)
                  </span>
                  <span class="text-[11px] font-medium text-cyan-300/80 tabular-nums">
                    · {crawlerRateText}
                  </span>
                </div>
              </div>
            </div>
          {/if}

          <!-- Content container underneath loader with padding to accommodate the absolute bar -->
          <div class="min-h-0 min-w-0 flex-1 bg-gray-950 {globalCollectorState.startsWith('global-aram:') || globalCollectorState === 'collecting' || agentOperations.some((op) => op.id === 'global-database') ? 'pt-8' : ''}">
            <div class="flex h-full min-h-0 flex-col">
              <div class="flex shrink-0 items-center justify-between border-b border-gray-800 bg-gray-950 px-4 pb-3 pt-4">
                <h1 class="flex items-center gap-2 text-lg font-bold tracking-wide text-gray-100">
                  <span class="icon-[heroicons--chart-bar] h-5 w-5 text-purple-400"></span>
                  Statistics
                </h1>
                <div class="flex items-center gap-2">
                  {#if agentOperations.length > 0}
                    <div class="relative group">
                      <button
                        type="button"
                        class="flex h-8 w-8 items-center justify-center rounded-md border border-cyan-700/60 bg-cyan-950/60 text-cyan-300 hover:border-cyan-500 hover:text-cyan-200 transition-colors"
                        title={agentOperations.map((o) => `${o.message}${o.current !== null && o.total !== null ? ` (${o.current}/${o.total})` : ''}`).join('\n')}
                        aria-label="Background agent operation in progress"
                      >
                        <span class="relative flex h-3 w-3">
                          <span class="absolute inline-flex h-full w-full animate-ping rounded-full bg-cyan-400 opacity-75"></span>
                          <span class="relative inline-flex h-3 w-3 rounded-full bg-cyan-500"></span>
                        </span>
                      </button>
                      <!-- Tooltip dropdown on hover/focus -->
                      <div class="pointer-events-none absolute left-0 top-9 z-30 hidden w-64 rounded-lg border border-cyan-800/80 bg-gray-950/95 p-2.5 text-[11px] text-cyan-100 shadow-2xl backdrop-blur group-hover:block group-focus-within:block">
                        <div class="font-semibold text-cyan-300 mb-1 flex items-center justify-between">
                          <span>Active Operations</span>
                          <span class="text-[10px] text-cyan-400">{agentOperations.length}</span>
                        </div>
                        {#each agentOperations as operation (operation.id)}
                          <div class="py-1 border-t border-gray-800">
                            <div class="flex items-center justify-between gap-2">
                              <span class="truncate">{operation.message}</span>
                              {#if operation.current !== null && operation.total !== null}
                                <span class="tabular-nums text-cyan-300 shrink-0">{operation.current}/{operation.total}</span>
                              {/if}
                            </div>
                            {#if operation.current !== null && operation.total !== null && operation.total > 0}
                              <div class="mt-1 h-1 overflow-hidden rounded-full bg-cyan-950">
                                <div
                                  class="h-full rounded-full bg-cyan-400 transition-[width] duration-200"
                                  style={`width: ${Math.min(100, Math.round((operation.current / operation.total) * 100))}%`}
                                ></div>
                              </div>
                            {/if}
                          </div>
                        {/each}
                      </div>
                    </div>
                  {/if}

                  <div class="flex rounded-lg border border-gray-700 bg-gray-900 p-0.5 text-xs">
                    {#each [["personal", "Mine"], ["compare", "Compare"]] as option}
                      <button type="button" class="rounded-md px-2.5 py-1.5 transition-colors {statisticsScope === option[0] ? 'bg-cyan-700 text-white' : 'text-gray-400 hover:text-white'}" onclick={() => handleStatisticsScopeChange(option[0] as "personal" | "compare")}>{option[1]}</button>
                    {/each}
                  </div>

                  {#if globalHasPatchMismatch}
                    <button
                      type="button"
                      onclick={() => {
                        statisticsConfigOpen = true;
                      }}
                      title="Global database contains matches from older patches. Click to inspect or reset."
                      class="flex cursor-pointer items-center gap-1.5 rounded-md border border-amber-500/50 bg-amber-950/40 px-2.5 py-1.5 text-xs font-medium text-amber-300 transition-colors hover:border-amber-400 hover:bg-amber-950/70"
                    >
                      <span class="icon-[heroicons--exclamation-triangle] h-4 w-4 text-amber-400"></span>
                      <span>Outdated patch data ({globalOutdatedGames.toLocaleString()})</span>
                    </button>
                  {/if}

                  <div class="relative">
                    <button
                      type="button"
                      class="relative flex h-8 w-8 shrink-0 items-center justify-center rounded-md border p-0 transition-colors {globalHasPatchMismatch ? 'border-amber-500/60 bg-amber-950/30 text-amber-300 hover:text-amber-100' : 'border-gray-700 text-gray-400 hover:text-white'}"
                      title="Global database settings"
                      aria-label="Global database settings"
                      onclick={() => {
                        statisticsConfigOpen = !statisticsConfigOpen;
                        if (!statisticsConfigOpen) confirmingGlobalReset = false;
                      }}
                    >
                      <span class="icon-[heroicons--cog-6-tooth] h-4 w-4"></span>
                      {#if globalHasPatchMismatch}
                        <span class="absolute -right-1 -top-1 flex h-2.5 w-2.5">
                          <span class="absolute inline-flex h-full w-full animate-ping rounded-full bg-amber-400 opacity-75"></span>
                          <span class="relative inline-flex h-2.5 w-2.5 rounded-full bg-amber-500"></span>
                        </span>
                      {/if}
                    </button>
                    {#if statisticsConfigOpen}
                      <div class="absolute right-0 top-9 z-30 w-72 rounded-lg border border-gray-700 bg-gray-900 p-3 shadow-xl">
                        {#if globalHasPatchMismatch}
                          <div class="mb-3 rounded-md border border-amber-500/40 bg-amber-950/40 p-2 text-[11px] text-amber-200">
                            <div class="flex items-center gap-1.5 font-semibold text-amber-300">
                              <span class="icon-[heroicons--exclamation-triangle] h-4 w-4 shrink-0 text-amber-400"></span>
                              <span>Cross-patch games detected</span>
                            </div>
                            <p class="mt-1 text-[10px] leading-relaxed text-amber-200/80">
                              {#if globalCurrentPatch}
                                Current patch is <span class="font-semibold text-amber-100">v{globalCurrentPatch}</span>, but {globalOutdatedGames.toLocaleString()} stored {globalOutdatedGames === 1 ? "game is" : "games are"} from older patches.
                              {:else}
                                Stored games span multiple patches ({globalOutdatedGames.toLocaleString()} from older patches).
                              {/if}
                              Resetting is recommended after major ARAM Mayhem updates.
                            </p>
                            {#if globalPatches.length > 0}
                              <div class="mt-1.5 flex flex-wrap gap-1">
                                {#each globalPatches as entry (entry.patch)}
                                  <span class="rounded border px-1.5 py-0.5 text-[10px] tabular-nums {globalCurrentPatch && entry.patch === globalCurrentPatch ? 'border-emerald-600/50 bg-emerald-950/50 text-emerald-300' : 'border-amber-600/40 bg-amber-950/60 text-amber-300'}">
                                    {entry.patch === "unknown" ? "unknown" : `v${entry.patch}`}: {entry.games.toLocaleString()}
                                  </span>
                                {/each}
                              </div>
                            {/if}
                          </div>
                        {/if}

                        <label class="text-[11px] uppercase tracking-wider text-gray-500" for="global-aram-limit">Global ARAM game limit</label>
                        <input id="global-aram-limit" type="number" min="1" max="100000" bind:value={globalLimit} class="mt-1 w-full rounded border border-gray-700 bg-gray-950 px-2 py-1.5 text-xs text-gray-100" />
                        <p class="mt-1 text-[10px] text-gray-400">
                          {globalGames.toLocaleString()}/{globalLimit.toLocaleString()} games{#if globalCurrentPatch} · Patch v{globalCurrentPatch}{/if}
                        </p>
                        <p class="mt-1 break-words text-[10px] text-cyan-300">Status: {globalCollectorState}{#if globalQueuedJobs > 0} · {globalQueuedJobs.toLocaleString()} queued{/if}</p>
                        <label class="mt-3 flex items-center gap-2 text-[11px] text-gray-300">
                          <input type="checkbox" checked={hideLowPickrate} onchange={handleHideLowPickrateChange} class="rounded border-gray-700 bg-gray-950 text-purple-600 focus:ring-purple-500" />
                          Hide low pick-rate options (&lt;1% champs, &lt;0.2% items)
                        </label>
                        <button type="button" class="mt-2 w-full cursor-pointer rounded bg-purple-600 px-2 py-1.5 text-xs font-medium text-white transition-colors hover:bg-purple-500 disabled:opacity-50" disabled={savingGlobalLimit || resettingGlobalDb} onclick={saveGlobalStatisticsLimit}>{savingGlobalLimit ? "Saving..." : "Save settings"}</button>

                        <div class="mt-3 border-t border-gray-800 pt-2.5">
                          {#if !confirmingGlobalReset}
                            <button
                              type="button"
                              disabled={resettingGlobalDb || savingGlobalLimit}
                              onclick={() => (confirmingGlobalReset = true)}
                              class="flex w-full cursor-pointer items-center justify-center gap-1.5 rounded border border-red-800/70 bg-red-950/30 px-2 py-1.5 text-xs font-medium text-red-300 transition-colors hover:border-red-600 hover:bg-red-950/60 hover:text-red-200 disabled:opacity-50"
                            >
                              <span class="icon-[heroicons--trash] h-3.5 w-3.5"></span>
                              <span>Reset Database &amp; Search Tree</span>
                            </button>
                          {:else}
                            <p class="mb-2 text-[10px] leading-snug text-red-300">
                              Delete all {globalGames.toLocaleString()} global ARAM Mayhem games and reset the cached account search tree?
                            </p>
                            <div class="flex gap-1.5">
                              <button
                                type="button"
                                disabled={resettingGlobalDb}
                                onclick={handleResetGlobalDatabase}
                                class="flex-1 cursor-pointer rounded bg-red-600 px-2 py-1.5 text-xs font-semibold text-white transition-colors hover:bg-red-500 disabled:opacity-50"
                              >
                                {resettingGlobalDb ? "Resetting..." : "Confirm Reset"}
                              </button>
                              <button
                                type="button"
                                disabled={resettingGlobalDb}
                                onclick={() => (confirmingGlobalReset = false)}
                                class="cursor-pointer rounded border border-gray-700 bg-gray-800 px-2.5 py-1.5 text-xs text-gray-300 transition-colors hover:bg-gray-700 disabled:opacity-50"
                              >
                                Cancel
                              </button>
                            </div>
                          {/if}
                        </div>
                      </div>
                    {/if}
                  </div>

                  <div class="flex rounded-lg border border-gray-700 bg-gray-900 p-0.5 text-xs">
                    <button type="button" class="rounded-md px-3 py-1.5 transition-colors {statisticsPerspective === 'champions' ? 'bg-purple-600 text-white' : 'text-gray-400 hover:text-white'}" onclick={() => handleStatisticsPerspectiveChange("champions")}>Champions</button>
                    <button type="button" class="rounded-md px-3 py-1.5 transition-colors {statisticsPerspective === 'builds' ? 'bg-purple-600 text-white' : 'text-gray-400 hover:text-white'}" onclick={() => handleStatisticsPerspectiveChange("builds")}>Builds</button>
                  </div>
                </div>
              </div>
              <div class="min-h-0 flex-1">
                {#if statisticsPerspective === "champions"}
                  <ProfileStatisticsView showHeader={false} scope={statisticsScope} hideLowPickrate={hideLowPickrate} refreshKey={statisticsRefreshKey} />
                {:else}
                  <BuildStatisticsView scope={statisticsScope} hideLowPickrate={hideLowPickrate} refreshKey={statisticsRefreshKey} />
                {/if}
              </div>
            </div>
          </div>
        {/if}
      </div>
    </div>
    {:else}
      <!-- Profile tab view: Match History & Summoner Profile -->
      <div class="flex min-h-0 flex-1 overflow-hidden">
        <nav class="flex w-16 shrink-0 flex-col items-center gap-2 border-r border-gray-800 bg-gray-950 py-4">
          <button
            type="button"
            title="Match history"
            aria-label="Match history"
            class="flex h-11 w-11 items-center justify-center rounded-xl bg-purple-600 text-white"
          >
            <span class="icon-[heroicons--clock] h-5 w-5"></span>
          </button>
        </nav>
        <div class="min-h-0 min-w-0 flex-1 flex flex-col bg-gray-950">
          <MatchHistoryView />
        </div>
      </div>
    {/if}
  </div>
</div>
