<script lang="ts">
  import {
    LayoutGroup,
    MotionAside,
    type MotionTransition,
  } from "@humanspeak/svelte-motion";
  import SidebarSkeleton from "./SidebarSkeleton.svelte";
  import SavedRunesList from "./SavedRunesList.svelte";
  import ClientRunesList from "./ClientRunesList.svelte";
  import ChampionSelectModal from "../ChampionSelectModal.svelte";
  import type {
    ClientRunePageSummary,
    RuneSetSummary,
  } from "../../types/runes";
  import type { ChampionSummary } from "../../types/champions";

  let {
    isOpen = $bindable(true),
    clientPages = [],
    savedRunesets = [],
    activeCompanionName = null,
    selectedKey = "",
    selectedChampion = null,
    allChampions = [],
    loading = false,
    onSelectPage,
    onImportPage,
    onDeletePage,
    onChangeRunesetChampion,
  }: {
    isOpen?: boolean;
    clientPages: ClientRunePageSummary[];
    savedRunesets: RuneSetSummary[];
    activeCompanionName: string | null;
    selectedKey: string;
    selectedChampion?: ChampionSummary | null;
    allChampions?: ChampionSummary[];
    loading: boolean;
    onSelectPage: (key: string) => void;
    onImportPage: (pageId: number) => void;
    onDeletePage: (runesetId: number) => void;
    onChangeRunesetChampion?: (runesetId: number, championId: number) => void;
  } = $props();

  // Modal state for editing saved runeset champion
  let editingRunesetId = $state<number | null>(null);
  let editingRunesetCurrentChampId = $state<number | null>(null);
  let isRunesetChampionModalOpen = $state<boolean>(false);

  // Filter saved runes:
  // - Shows saved runes for selectedChampion (or universal runes if selectedChampion is null)
  // - If the currently selected rune page (selectedKey) is from a different champion, prepend it before the current champion's saved runes!
  const filteredSavedRunes = $derived.by(() => {
    let list: RuneSetSummary[];
    if (selectedChampion && selectedChampion.id > 0) {
      list = savedRunesets.filter(
        (r) =>
          r.champion_id === selectedChampion.id ||
          !r.champion_id ||
          r.champion_id === 0,
      );
    } else {
      list = savedRunesets.filter((r) => !r.champion_id || r.champion_id === 0);
    }

    // If the active/selected page is a saved runeset from a different champion, prepend it before the current champion's saved runes
    if (selectedKey.startsWith("saved:")) {
      const activeId = parseInt(selectedKey.slice("saved:".length), 10);
      const activeRuneset = savedRunesets.find((r) => r.id === activeId);
      if (activeRuneset && !list.some((r) => r.id === activeId)) {
        list = [activeRuneset, ...list];
      }
    }

    return list;
  });

  function handleOpenChangeChampionModal(
    runesetId: number,
    currentChampId: number,
  ) {
    editingRunesetId = runesetId;
    editingRunesetCurrentChampId = currentChampId;
    isRunesetChampionModalOpen = true;
  }

  function handleSelectRunesetChampion(champ: ChampionSummary | null) {
    if (editingRunesetId !== null) {
      const champId = champ ? champ.id : 0;
      onChangeRunesetChampion?.(editingRunesetId, champId);
    }
  }

  let layoutTransition: MotionTransition = $derived({
    x: { duration: 0.5, ease: [0.16, 1, 0.3, 1] },
    width: { duration: 0.5, ease: [0.16, 1, 0.3, 1] },
    y: { duration: 0.5, ease: "linear" },
    height: { duration: 0.5, ease: "linear" },
    opacity: { duration: 0.5, ease: "linear" },
  });
</script>

<LayoutGroup id="sidebar-rail">
  <MotionAside
    key="runes-sidebar"
    animate={{ width: isOpen ? 330 : 130 }}
    transition={layoutTransition}
    class="h-full shrink-0 border-r border-gray-800 bg-gray-900/60 overflow-hidden select-none z-20"
  >
    <div
      class="h-full w-full p-2.5 flex flex-col gap-3 select-none overflow-y-auto overflow-x-hidden min-w-0"
    >
      {#if loading}
        <!-- Skeleton Loaders -->
        <SidebarSkeleton />
      {:else}
        <div id="saved-runes-container" class="space-y-4 w-full min-w-0">
          <!-- Saved Companion Runes -->
          <SavedRunesList
            items={filteredSavedRunes}
            {selectedKey}
            {activeCompanionName}
            {allChampions}
            {isOpen}
            onSelect={(id) => onSelectPage(`saved:${id}`)}
            onDelete={onDeletePage}
            onChangeChampion={handleOpenChangeChampionModal}
          />

          <!-- Client Pages -->
          <ClientRunesList
            items={clientPages}
            {selectedKey}
            {isOpen}
            onSelect={(id) => onSelectPage(`client:${id}`)}
            onImport={onImportPage}
          />
        </div>
      {/if}
    </div>
  </MotionAside>
</LayoutGroup>

<!-- Flowbite Champion Select Modal for changing saved runeset champion -->
<ChampionSelectModal
  isOpen={isRunesetChampionModalOpen}
  title="Change Rune Page Champion"
  currentChampionId={editingRunesetCurrentChampId}
  onSelect={handleSelectRunesetChampion}
  onClose={() => (isRunesetChampionModalOpen = false)}
/>
