<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import TitleBar from "./lib/components/TitleBar.svelte";
  import RunesSidebar from "./lib/components/sidebar/RunesSidebar.svelte";
  import RuneCanvas from "./lib/components/runes/RuneCanvas.svelte";
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
  } from "./lib/api/champions";
  import type {
    ClientRunePageSummary,
    RuneSetSummary,
    RuneTreePayload,
    SaveRuneTreePayload,
  } from "./lib/types/runes";
  import type { ChampionSummary } from "./lib/types/champions";

  let clientPages = $state<ClientRunePageSummary[]>([]);
  let savedRunesets = $state<RuneSetSummary[]>([]);
  let activeCompanionName = $state<string | null>(null);
  let selectedKey = $state<string>("");
  let selectedChampion = $state<ChampionSummary | null>(null);
  let allChampions = $state<ChampionSummary[]>([]);

  let currentTree = $state<RuneTreePayload | null>(null);
  let sidebarLoading = $state(true);
  let treeLoading = $state(true);
  let runesSidebarRef = $state<RunesSidebar | null>(null);
  let isSidebarOpen = $state<boolean>(false);

  function toggleSidebar() {
    isSidebarOpen = !isSidebarOpen;
  }

  async function loadChampions() {
    try {
      allChampions = await getChampions();
    } catch (err) {
      console.error("[Champions Load Error]", err);
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
      console.error("[Restore Selected Champion Error]", err);
    }
  }

  async function handleSelectChampionFilter(champ: ChampionSummary | null) {
    selectedChampion = champ;
    try {
      await setSelectedChampionId(champ ? champ.id : null);
    } catch (err) {
      console.error("[Save Selected Champion Error]", err);
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
        if (candidateKey.startsWith("saved:") && savedRunesets.some((s) => `saved:${s.id}` === candidateKey)) {
          keyToSelect = candidateKey;
        } else if (candidateKey.startsWith("client:") && clientPages.some((c) => `client:${c.id}` === candidateKey)) {
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
      console.error("[Tree Load Error]", err);
    } finally {
      treeLoading = false;
    }
  }

  async function handleSelectPage(key: string) {
    try {
      await setSelectedRuneKey(key);
    } catch (err) {
      console.error("[Save Selected Rune Key Error]", err);
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
      runesSidebarRef?.refreshChampions();
    } catch (err) {
      console.error("[Import Error]", err);
    }
  }

  async function handleExportPage(runesetId: number) {
    try {
      await exportSavedRunePage(runesetId);
      await loadOverview();
      runesSidebarRef?.refreshProfile();
    } catch (err) {
      console.error("[Export Error]", err);
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
      runesSidebarRef?.refreshChampions();
    } catch (err) {
      console.error("[Delete Error]", err);
    }
  }

  async function handleUpdateRunesetChampion(runesetId: number, championId: number) {
    try {
      await updateRunesetChampion(runesetId, championId);
      await loadOverview();
      if (selectedKey === `saved:${runesetId}`) {
        await loadTreeForKey(`saved:${runesetId}`);
      }
      runesSidebarRef?.refreshChampions();
    } catch (err) {
      console.error("[Update Runeset Champion Error]", err);
    }
  }

  async function handleSaveTree(data: {
    name: string;
    primaryStyleId: number;
    subStyleId: number;
    selectedPerks: number[];
    championId: number;
  }) {
    const payload: SaveRuneTreePayload = {
      runeset_id: currentTree?.runeset_id,
      page_id: currentTree?.page_id,
      page_name: data.name,
      primary_style_id: data.primaryStyleId,
      sub_style_id: data.subStyleId,
      selected_perks: data.selectedPerks,
      champion_id: data.championId,
    };

    const savedId = await saveRuneTree(payload);
    const savedKey = `saved:${savedId}`;
    await setSelectedRuneKey(savedKey);
    await loadOverview(savedKey);
    await loadTreeForKey(savedKey);
    runesSidebarRef?.refreshChampions();
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
        runesSidebarRef?.refreshChampions();
      });
      unlistenDisconnected = await listen("lcu:disconnected", async () => {
        clientPages = [];
        activeCompanionName = null;
        await loadOverview(selectedKey);
      });
    } catch (err) {
      console.error("[App] Failed to attach Tauri event listeners:", err);
    }
  });

  onDestroy(() => {
    if (unlistenRunes) unlistenRunes();
    if (unlistenConnected) unlistenConnected();
    if (unlistenDisconnected) unlistenDisconnected();
  });
</script>

<div class="h-screen w-screen flex flex-col overflow-hidden bg-gray-900 text-gray-100 antialiased selection:bg-purple-600 selection:text-white">
  <!-- Draggable Custom Title Bar with Sidebar Toggle -->
  <TitleBar {isSidebarOpen} onToggleSidebar={toggleSidebar} />

  <!-- Main App Shell -->
  <div id="app-shell" class="flex-1 flex overflow-hidden bg-gray-900 text-gray-100 relative">
    <!-- Collapsible Sidebar Component -->
    <RunesSidebar
      bind:this={runesSidebarRef}
      bind:isOpen={isSidebarOpen}
      {clientPages}
      {savedRunesets}
      {activeCompanionName}
      {selectedKey}
      {selectedChampion}
      activeRuneChampionId={currentTree?.champion_id || 0}
      loading={sidebarLoading}
      onSelectPage={handleSelectPage}
      onImportPage={handleImportPage}
      onDeletePage={handleDeletePage}
      onSelectChampionFilter={handleSelectChampionFilter}
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
  </div>
</div>
