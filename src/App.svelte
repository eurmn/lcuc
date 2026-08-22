<script lang="ts">
  import { onMount } from "svelte";
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
    importClientPage,
    saveRuneTree,
  } from "./lib/api/runes";
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
  let sidebarChampionFilter = $state<ChampionSummary | null>(null);

  let currentTree = $state<RuneTreePayload | null>(null);
  let sidebarLoading = $state(true);
  let treeLoading = $state(true);
  let runesSidebarRef = $state<RunesSidebar | null>(null);

  async function loadOverview(preserveKey?: string): Promise<string> {
    try {
      sidebarLoading = true;
      const overview = await getRunesOverview();
      clientPages = overview.client_pages || [];
      savedRunesets = overview.saved_runesets || [];
      activeCompanionName = overview.active_companion_name || null;

      let keyToSelect = preserveKey || selectedKey;
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
    await loadTreeForKey(key);
  }

  async function handleImportPage(pageId: number) {
    try {
      const newId = await importClientPage(pageId);
      await loadOverview(`saved:${newId}`);
      await loadTreeForKey(`saved:${newId}`);
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
        await loadTreeForKey(nextKey);
      } else {
        currentTree = null;
      }
    } catch (err) {
      console.error("[Delete Error]", err);
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
    };

    const savedId = await saveRuneTree(payload);
    await loadOverview(`saved:${savedId}`);
    await loadTreeForKey(`saved:${savedId}`);
  }


  onMount(async () => {

    const key = await loadOverview();
    if (key) {
      await loadTreeForKey(key);
    }
  });
</script>

<div class="h-screen w-screen flex flex-col overflow-hidden bg-gray-900 text-gray-100 antialiased selection:bg-purple-600 selection:text-white">
  <!-- Draggable Custom Title Bar -->
  <TitleBar />

  <!-- Main App Shell -->
  <div id="app-shell" class="flex-1 flex overflow-hidden bg-gray-900 text-gray-100 relative">
    <!-- Left Sidebar: Profile & Rune Lists -->
    <RunesSidebar
      bind:this={runesSidebarRef}
      {clientPages}
      {savedRunesets}
      {activeCompanionName}
      {selectedKey}
      loading={sidebarLoading}
      onSelectPage={handleSelectPage}
      onImportPage={handleImportPage}
      onDeletePage={handleDeletePage}
      onSelectChampionFilter={(c) => (sidebarChampionFilter = c)}
    />

    <!-- Center Canvas: Active / Selected Rune Tree -->
    <RuneCanvas
      tree={currentTree}
      loading={treeLoading}
      championFilter={sidebarChampionFilter}
      onSave={handleSaveTree}
      onExport={handleExportPage}
      onImport={handleImportPage}
    />


  </div>
</div>
