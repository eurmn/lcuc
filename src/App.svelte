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

  let clientPages = $state<ClientRunePageSummary[]>([]);
  let savedRunesets = $state<RuneSetSummary[]>([]);
  let activeCompanionName = $state<string | null>(null);
  let selectedKey = $state<string>("");
  let selectedChampion = $state<ChampionSummary | null>(null);
  let allChampions = $state<ChampionSummary[]>([]);
  let activeTab = $state<"builds" | "profile">("builds");
  let profileTab = $state<"history" | "statistics">("history");
  let statisticsPerspective = $state<"champions" | "builds">("champions");

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
        <div class="min-h-0 min-w-0 flex-1">
          {#if profileTab === "history"}
            <MatchHistoryView />
          {:else}
            <div class="flex h-full min-h-0 flex-col">
              <div class="flex shrink-0 items-center justify-between border-b border-gray-800 bg-gray-950 px-4 pb-3 pt-4">
                <h1 class="flex items-center gap-2 text-lg font-bold tracking-wide text-gray-100">
                  <span class="icon-[heroicons--chart-bar] h-5 w-5 text-purple-400"></span>
                  Statistics
                </h1>
                <div class="flex rounded-lg border border-gray-700 bg-gray-900 p-0.5 text-xs">
                  <button type="button" class="rounded-md px-3 py-1.5 transition-colors {statisticsPerspective === 'champions' ? 'bg-purple-600 text-white' : 'text-gray-400 hover:text-white'}" onclick={() => handleStatisticsPerspectiveChange("champions")}>Champions</button>
                  <button type="button" class="rounded-md px-3 py-1.5 transition-colors {statisticsPerspective === 'builds' ? 'bg-purple-600 text-white' : 'text-gray-400 hover:text-white'}" onclick={() => handleStatisticsPerspectiveChange("builds")}>Builds</button>
                </div>
              </div>
              <div class="min-h-0 flex-1">
                {#if statisticsPerspective === "champions"}
                  <ProfileStatisticsView showHeader={false} />
                {:else}
                  <BuildStatisticsView />
                {/if}
              </div>
            </div>
          {/if}
        </div>
      </div>
    {/if}
  </div>
</div>
