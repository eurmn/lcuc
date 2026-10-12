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
    setGlobalAramLimit,
    setGlobalAramWorkers,
    type AgentOperation,
  } from "./lib/api/agent";

  let clientPages = $state<ClientRunePageSummary[]>([]);
  let savedRunesets = $state<RuneSetSummary[]>([]);
  let activeCompanionName = $state<string | null>(null);
  let selectedKey = $state<string>("");
  let selectedChampion = $state<ChampionSummary | null>(null);
  let allChampions = $state<ChampionSummary[]>([]);
  let activeTab = $state<"builds" | "profile">("builds");
  let profileTab = $state<"history" | "statistics">("history");
  let statisticsPerspective = $state<"champions" | "builds">("champions");
  let statisticsScope = $state<"personal" | "compare">("personal");
  let statisticsConfigOpen = $state(false);
  let globalLimit = $state(1000);
  let globalWorkers = $state(3);
  let globalGames = $state(0);
  let globalCollectorState = $state("unknown");
  let globalQueuedJobs = $state(0);
  let agentOperations = $state<AgentOperation[]>([]);
  let savingGlobalLimit = $state(false);
  let hideLowPickrate = $state(false);

  let currentTree = $state<RuneTreePayload | null>(null);
  let sidebarLoading = $state(true);
  let treeLoading = $state(true);
  let topNavBarRef = $state<TopNavBar | null>(null);
  let isSidebarOpen = $state<boolean>(true);

  onMount(() => {
    const savedTab = localStorage.getItem("lcu-active-tab");
    if (savedTab === "builds" || savedTab === "profile") {
      activeTab = savedTab;
    }
    const savedProfileTab = localStorage.getItem("lcu-profile-tab");
    if (savedProfileTab === "history" || savedProfileTab === "statistics") {
      profileTab = savedProfileTab;
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
          globalWorkers = config.workers ?? 3;
        }
        globalGames = config.games;
        globalCollectorState = config.collector_state;
        globalQueuedJobs = config.queued_jobs;
        agentOperations = config.operations ?? [];

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

  function handleTabChange(tab: "builds" | "profile") {
    activeTab = tab;
    localStorage.setItem("lcu-active-tab", tab);
  }

  function handleProfileTabChange(tab: "history" | "statistics") {
    profileTab = tab;
    localStorage.setItem("lcu-profile-tab", tab);
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
      const [limitResult, workersResult] = await Promise.all([
        setGlobalAramLimit(Math.max(1, Math.min(100000, Math.floor(globalLimit)))),
        setGlobalAramWorkers(Math.max(1, Math.min(16, Math.floor(globalWorkers)))),
      ]);
      globalLimit = limitResult.limit;
      globalWorkers = workersResult.workers;
      statisticsConfigOpen = false;
    } catch (error) {
      logger.error("frontend.app", "Failed to save global statistics settings", error);
    } finally {
      savingGlobalLimit = false;
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
    {:else}
      <div class="flex min-h-0 flex-1 overflow-hidden">
        <nav class="flex w-16 shrink-0 flex-col items-center gap-2 border-r border-gray-800 bg-gray-950 py-4">
          <button
            type="button"
            title="Match history"
            aria-label="Match history"
            onclick={() => handleProfileTabChange("history")}
            class="flex h-11 w-11 items-center justify-center rounded-xl transition-colors {profileTab === 'history' ? 'bg-purple-600 text-white' : 'text-gray-500 hover:bg-gray-800 hover:text-gray-200'}"
          >
            <span class="icon-[heroicons--clock] h-5 w-5"></span>
          </button>
          <button
            type="button"
            title="Statistics"
            aria-label="Statistics"
            onclick={() => handleProfileTabChange("statistics")}
            class="flex h-11 w-11 items-center justify-center rounded-xl transition-colors {profileTab === 'statistics' ? 'bg-purple-600 text-white' : 'text-gray-500 hover:bg-gray-800 hover:text-gray-200'}"
          >
            <span class="icon-[heroicons--chart-bar] h-5 w-5"></span>
          </button>
        </nav>
        <div class="relative min-h-0 min-w-0 flex-1 flex flex-col bg-gray-950">
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

              <!-- 2. Status row underneath the bar: left-aligned counter, percentage, and progress donut with more padding and matching bg color -->
              <div class="flex items-center justify-start px-4 pt-2 pb-1 text-xs">
                <div class="flex items-center gap-2.5 rounded-md px-2 py-1">
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
                </div>
              </div>
            </div>
          {/if}

          <!-- Content container underneath loader with padding to accommodate the absolute bar -->
          <div class="min-h-0 min-w-0 flex-1 bg-gray-950 {globalCollectorState.startsWith('global-aram:') || globalCollectorState === 'collecting' || agentOperations.some((op) => op.id === 'global-database') ? 'pt-8' : ''}">
            {#if profileTab === "history"}
            <MatchHistoryView />
          {:else}
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

                  <div class="relative">
                    <button type="button" class="flex h-8 w-8 shrink-0 items-center justify-center rounded-md border border-gray-700 p-0 text-gray-400 hover:text-white" title="Global database settings" aria-label="Global database settings" onclick={() => (statisticsConfigOpen = !statisticsConfigOpen)}>
                      <span class="icon-[heroicons--cog-6-tooth] h-4 w-4"></span>
                    </button>
                    {#if statisticsConfigOpen}
                      <div class="absolute right-0 top-9 z-30 w-64 rounded-lg border border-gray-700 bg-gray-900 p-3 shadow-xl">
                        <label class="text-[11px] uppercase tracking-wider text-gray-500" for="global-aram-limit">Global ARAM game limit</label>
                        <input id="global-aram-limit" type="number" min="1" max="100000" bind:value={globalLimit} class="mt-1 w-full rounded border border-gray-700 bg-gray-950 px-2 py-1.5 text-xs text-gray-100" />
                        <label class="mt-2 block text-[11px] uppercase tracking-wider text-gray-500" for="global-aram-workers">Crawler worker agents</label>
                        <input id="global-aram-workers" type="number" min="1" max="16" bind:value={globalWorkers} class="mt-1 w-full rounded border border-gray-700 bg-gray-950 px-2 py-1.5 text-xs text-gray-100" />
                        <p class="mt-1 text-[10px] text-gray-400">{globalGames}/{globalLimit} games · {globalWorkers} workers</p>
                        <p class="mt-1 break-words text-[10px] text-cyan-300">Status: {globalCollectorState}{#if globalQueuedJobs > 0} · {globalQueuedJobs} queued{/if}</p>
                        <label class="mt-3 flex items-center gap-2 text-[11px] text-gray-300">
                          <input type="checkbox" checked={hideLowPickrate} onchange={handleHideLowPickrateChange} class="rounded border-gray-700 bg-gray-950 text-purple-600 focus:ring-purple-500" />
                          Hide low pick-rate options (&lt;1%)
                        </label>
                        <button type="button" class="mt-2 w-full rounded bg-purple-600 px-2 py-1.5 text-xs text-white disabled:opacity-50" disabled={savingGlobalLimit} onclick={saveGlobalStatisticsLimit}>{savingGlobalLimit ? "Saving..." : "Save settings"}</button>
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
                  <ProfileStatisticsView showHeader={false} scope={statisticsScope} hideLowPickrate={hideLowPickrate} />
                {:else}
                  <BuildStatisticsView scope={statisticsScope} hideLowPickrate={hideLowPickrate} />
                {/if}
              </div>
            </div>
          {/if}
        </div>
      </div>
    </div>
  {/if}
</div>
</div>
