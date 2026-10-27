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
    pauseGlobalAramCrawler,
    stopGlobalAramCrawler,
    type AgentOperation,
    type GlobalAramPatchBreakdown,
  } from "./lib/api/agent";
  import {
    clearStatisticsCache,
    syncStatisticsCachePatch,
  } from "./lib/utils/statistics-data";
  import { isLcuConnected as fetchIsLcuConnected } from "./lib/api/champ_select";
  import type { ChampSelectSession } from "./lib/types/champ_select";

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
  let globalLimit = $state(1000);
  let globalGames = $state(0);
  let globalCollectorState = $state("unknown");
  let globalQueuedJobs = $state(0);
  let globalCurrentPatch = $state<string | null>(null);
  let globalPatches = $state<GlobalAramPatchBreakdown[]>([]);
  let globalOutdatedGames = $state(0);
  let globalHasPatchMismatch = $state(false);
  let statisticsRefreshKey = $state(0);
  let agentOperations = $state<AgentOperation[]>([]);
  let hideLowPickrate = $state(false);
  let isLcuConnected = $state(false);
  let crawlerRateText = $state("0.0 match/min");
  let crawlerEtaText = $state("");
  let crawlerPaused = $state(false);

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
        globalLimit = config.limit;
        globalGames = config.games;
        globalCollectorState = config.collector_state;
        globalQueuedJobs = config.queued_jobs;
        globalCurrentPatch = config.current_patch ?? null;
        globalPatches = config.patches ?? [];
        globalOutdatedGames = config.outdated_games ?? 0;
        globalHasPatchMismatch = Boolean(config.has_patch_mismatch);
        agentOperations = config.operations ?? [];
        crawlerRateText = config.crawler_rate_text ?? "0.0 match/min";
        crawlerEtaText = config.crawler_eta_text ?? "";
        crawlerPaused = Boolean(config.crawler_paused);
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

  async function saveGlobalStatisticsLimit(targetLimit?: number) {
    try {
      const val = typeof targetLimit === "number" ? targetLimit : globalLimit;
      const limitResult = await setGlobalAramLimit(
        Math.max(1, Math.min(100000, Math.floor(val)))
      );
      globalLimit = limitResult.limit;
    } catch (error) {
      logger.error("frontend.app", "Failed to save global statistics settings", error);
    }
  }

  async function handleResetGlobalDatabase() {
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
      statisticsRefreshKey += 1;
    } catch (error) {
      logger.error("frontend.app", "Failed to reset global ARAM database", error);
    }
  }

  async function handlePauseCrawler(paused: boolean) {
    try {
      await pauseGlobalAramCrawler(paused);
      crawlerPaused = paused;
    } catch (e) {
      logger.error("frontend.app", `Failed to ${paused ? 'pause' : 'resume'} crawler`, e);
    }
  }

  async function handleStopCrawler() {
    try {
      await stopGlobalAramCrawler();
      globalLimit = globalGames;
    } catch (e) {
      logger.error("frontend.app", "Failed to stop crawler", e);
    }
  }

  function handleToggleHideLowPickrate() {
    hideLowPickrate = !hideLowPickrate;
    localStorage.setItem("lcu-statistics-hide-low-pickrate", String(hideLowPickrate));
    window.dispatchEvent(new Event("storage"));
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
  let unlistenChampSelect: UnlistenFn | null = null;

  onMount(async () => {
    try {
      isLcuConnected = await fetchIsLcuConnected();
    } catch {
      isLcuConnected = false;
    }
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
        isLcuConnected = true;
        await loadOverview(selectedKey);
        await loadChampions();
        topNavBarRef?.refreshChampions();
        topNavBarRef?.refreshProfile();
      });
      unlistenDisconnected = await listen("lcu:disconnected", async () => {
        isLcuConnected = false;
        clientPages = [];
        activeCompanionName = null;
        await loadOverview(selectedKey);
      });
      unlistenChampSelect = await listen<ChampSelectSession | null>(
        "lcu:champ-select-session",
        async (event) => {
          const data = event.payload;
          if (data && data.myTeam && data.myTeam.length > 0) {
            const myChampId = data.myTeam.find(
              (m) => m.cellId === data.localPlayerCellId
            )?.championId;
            if (myChampId && myChampId > 0 && (!selectedChampion || selectedChampion.id !== myChampId)) {
              if (allChampions.length === 0) {
                allChampions = await getChampions();
              }
              const matched = allChampions.find((c) => c.id === myChampId);
              if (matched) {
                selectedChampion = matched;
                await setSelectedChampionId(matched.id);
              }
            }
          }
        }
      );
    } catch (err) {
      logger.error("frontend.app", "Failed to attach Tauri event listeners", err);
    }
  });

  onDestroy(() => {
    if (unlistenRunes) unlistenRunes();
    if (unlistenConnected) unlistenConnected();
    if (unlistenDisconnected) unlistenDisconnected();
    if (unlistenChampSelect) unlistenChampSelect();
  });
</script>

<div
  class="h-screen w-screen flex flex-col overflow-hidden bg-gray-900 text-gray-100 antialiased selection:bg-purple-600 selection:text-white"
>
  <!-- Draggable Custom Title Bar with App Branding & Window Controls -->
  <TitleBar
    {activeTab}
    onTabChange={handleTabChange}
    {agentOperations}
    {globalHasPatchMismatch}
    {globalCollectorState}
    {globalGames}
    {globalLimit}
    {globalCurrentPatch}
    {globalPatches}
    {globalOutdatedGames}
    {globalQueuedJobs}
    {crawlerPaused}
    {crawlerRateText}
    {crawlerEtaText}
    {hideLowPickrate}
    onToggleHideLowPickrate={handleToggleHideLowPickrate}
    {isLcuConnected}
    onSaveLimit={saveGlobalStatisticsLimit}
    onResetGlobalDatabase={handleResetGlobalDatabase}
    onPauseCrawler={handlePauseCrawler}
    onStopCrawler={handleStopCrawler}
  />

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

  <!-- Global Crawler Progress Banner across all tabs -->
  {#if globalCollectorState.startsWith("global-aram:") || globalCollectorState === "collecting" || crawlerPaused || agentOperations.some((op) => op.id === "global-database")}
    {@const crawlerOp = agentOperations.find((op) => op.id === "global-database")}
    {@const currentCount = crawlerOp?.current ?? globalGames}
    {@const totalLimit = crawlerOp?.total ?? globalLimit}
    {@const progressPct = Math.min(100, Math.max(0, Math.round((currentCount / Math.max(1, totalLimit)) * 100)))}
    <div class="relative w-full z-20 select-none bg-gray-950 border-b border-cyan-900/30">
      <!-- Vivid thin progress bar line -->
      <div class="h-[2px] w-full bg-cyan-950/60 overflow-hidden">
        <div
          class="h-full bg-cyan-400 shadow-[0_0_8px_rgba(34,211,238,0.8)] transition-[width] duration-300 ease-out"
          style={`width: ${progressPct}%`}
        ></div>
      </div>

      <!-- Status row: counter, percentage, rate, ETA -->
      <div class="flex items-center justify-between px-4 py-1.5 text-xs bg-gray-950/90">
        <div class="flex items-center gap-2.5">
          <svg class="h-3.5 w-3.5 -rotate-90 shrink-0" viewBox="0 0 24 24">
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
          {#if crawlerEtaText}
            <span class="text-[11px] font-semibold text-amber-300/90 tabular-nums">
              · ETA {crawlerEtaText}
            </span>
          {/if}
          {#if crawlerPaused}
            <span class="px-1.5 py-0.2 rounded bg-amber-950/60 text-amber-400 border border-amber-800 text-[10px] font-bold uppercase tracking-wider">
              Paused
            </span>
          {/if}
        </div>
      </div>
    </div>
  {/if}

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
            <MayhemChampSelectView
              {selectedChampion}
              onSelectChampion={handleSelectChampionFilter}
              {hideLowPickrate}
              {isLcuConnected}
            />
          {:else}
          <!-- Content container for Statistics -->
          <div class="min-h-0 min-w-0 flex-1 bg-gray-950">
            <div class="flex h-full min-h-0 flex-col">
              <div class="flex shrink-0 items-center justify-between border-b border-gray-800 bg-gray-950 px-4 pb-3 pt-4">
                <h1 class="flex items-center gap-2 text-lg font-bold tracking-wide text-gray-100">
                  <span class="icon-[heroicons--chart-bar] h-5 w-5 text-purple-400"></span>
                  Statistics
                </h1>
                <div class="flex items-center gap-3">
                  <!-- Mine vs Compare switcher -->
                  <div class="flex rounded-lg border border-gray-700 bg-gray-900 p-0.5 text-xs">
                    {#each [["personal", "Mine"], ["compare", "Compare"]] as option}
                      <button type="button" class="rounded-md px-2.5 py-1.5 transition-colors {statisticsScope === option[0] ? 'bg-cyan-700 text-white' : 'text-gray-400 hover:text-white'}" onclick={() => handleStatisticsScopeChange(option[0] as "personal" | "compare")}>{option[1]}</button>
                    {/each}
                  </div>

                  <!-- Champions vs Builds switcher -->
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
