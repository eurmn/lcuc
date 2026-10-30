<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import {
    getChampSelectSession,
    swapChampSelectChampion,
    getLiveGameState,
    syncOverlayWindow,
    scanLiveAugmentCards,
    emitOverlayState,
    computeMayhemBuildRecommendation,
    getItemsCatalog,
  } from "../../api/champ_select";
  import { getChampions } from "../../api/champions";
  import type { ChampionSummary } from "../../types/champions";
  import type {
    ChampSelectSession,
    MayhemAugmentItem,
    MayhemBuildItem,
    MayhemBuildAugment,
    LiveGameState,
    BuildRecommendationResult,
  } from "../../types/champ_select";
  import type { AgentOperationDto } from "../../api/agent";
  import { logger } from "../../utils/logger";

  import MayhemRosterBar from "./MayhemRosterBar.svelte";
  import MayhemBuildEnginePanel from "./MayhemBuildEnginePanel.svelte";
  import MayhemBuildSimulator from "./MayhemBuildSimulator.svelte";
  import MayhemAugmentPicker from "./MayhemAugmentPicker.svelte";
  import MayhemCombosCard from "./MayhemCombosCard.svelte";
  import MayhemEncyclopediaModal from "./MayhemEncyclopediaModal.svelte";

  let {
    selectedChampion = null,
    onSelectChampion,
    hideLowPickrate = false,
    isLcuConnected = false,
    agentOperations = [],
  }: {
    selectedChampion?: ChampionSummary | null;
    onSelectChampion?: (champ: ChampionSummary | null) => void;
    hideLowPickrate?: boolean;
    isLcuConnected?: boolean;
    agentOperations?: AgentOperationDto[];
  } = $props();

  let session = $state<ChampSelectSession | null>(null);
  let allChampions = $state<ChampionSummary[]>([]);
  let selectedChampionId = $state<number>(22);
  let championSearchQuery = $state("");
  let championPickerOpen = $state(false);

  let userBuildSlots = $state<(MayhemBuildItem | null)[]>(Array(6).fill(null));
  let selectedBuildSlotIndex = $state<number | null>(null);
  let pickerSlots = $state<(MayhemAugmentItem | null)[]>([null, null, null, null]);
  let augmentIds = $state<string[]>([]);
  let itemsCatalog = $state<Record<number, {
    name: string;
    icon_url: string;
    item_class?: string;
    description?: string;
    price_total: number;
    categories: string[];
  }>>({});
  let buildEngineRec = $state<BuildRecommendationResult | null>(null);
  let loadingRec = $state(false);

  let overlayEnabled = $state<boolean>(true);
  let liveGameState = $state<LiveGameState | null>(null);
  let isEncyclopediaOpen = $state(false);
  let scanToast = $state<{ message: string; isError: boolean } | null>(null);
  let scanToastTimeout: ReturnType<typeof setTimeout> | null = null;

  function showScanToast(message: string, isError = false) {
    if (scanToastTimeout) clearTimeout(scanToastTimeout);
    scanToast = { message, isError };
    scanToastTimeout = setTimeout(() => {
      scanToast = null;
      scanToastTimeout = null;
    }, 4000);
  }

  let unlistenSession: UnlistenFn | null = null;
  let unlistenPhase: UnlistenFn | null = null;
  let unlistenLiveState: UnlistenFn | null = null;
  let unlistenOverlaySync: UnlistenFn | null = null;

  const WARP_AUGMENTS = [
    "Marksmage",
    "ADAPt",
    "Big Shot",
    "Apex Inventor",
    "Vulnerability",
    "Goliath",
    "Draw Your Sword",
  ];

  const currentSelectedChampion = $derived(
    allChampions.find((c) => c.id === selectedChampionId) ?? null
  );

  const filteredChampions = $derived(
    championSearchQuery.trim() === ""
      ? allChampions
      : allChampions.filter((c) =>
          c.name.toLowerCase().includes(championSearchQuery.toLowerCase())
        )
  );

  const isInChampSelect = $derived(
    Boolean(
      session &&
        session.myTeam &&
        session.myTeam.length > 0 &&
        (session.queueId === 2400 || session.benchEnabled)
    )
  );

  const userBuildItemIds = $derived(
    userBuildSlots.filter((i): i is MayhemBuildItem => i !== null).map((i) => i.id)
  );

  const knownItemMap = $derived.by(() => {
    const map = new Map<number, MayhemBuildItem>();
    for (const [idStr, info] of Object.entries(itemsCatalog)) {
      const id = Number(idStr);
      map.set(id, {
        id,
        name: info.name,
        icon_url: info.icon_url,
        price_total: info.price_total,
        item_class: info.item_class,
        description: info.description,
      });
    }
    if (!buildEngineRec) return map;
    if (buildEngineRec.recommended_boots) {
      const b = buildEngineRec.recommended_boots;
      if (!map.has(b.item_id)) {
        map.set(b.item_id, {
          id: b.item_id,
          name: `Boot #${b.item_id}`,
          icon_url: `https://raw.communitydragon.org/latest/plugins/rcp-be-lol-game-data/global/default/assets/items/icons2d/${b.item_id}.png`.toLowerCase(),
          price_total: 0,
        });
      }
    }
    for (const it of buildEngineRec.next_items ?? []) {
      if (!map.has(it.item_id)) {
        map.set(it.item_id, {
          id: it.item_id,
          name: `Item #${it.item_id}`,
          icon_url: `https://raw.communitydragon.org/latest/plugins/rcp-be-lol-game-data/global/default/assets/items/icons2d/${it.item_id}.png`.toLowerCase(),
          price_total: 0,
        });
      }
    }
    for (const it of buildEngineRec.completed_build ?? []) {
      const id = typeof it === "number" ? it : it.item_id;
      if (!map.has(id)) {
        map.set(id, {
          id,
          name: `Item #${id}`,
          icon_url: `https://raw.communitydragon.org/latest/plugins/rcp-be-lol-game-data/global/default/assets/items/icons2d/${id}.png`.toLowerCase(),
          price_total: 0,
        });
      }
    }
    for (const it of buildEngineRec.alternative_items ?? []) {
      const id = typeof it === "number" ? it : it.item_id;
      if (!map.has(id)) {
        map.set(id, {
          id,
          name: `Item #${id}`,
          icon_url: `https://raw.communitydragon.org/latest/plugins/rcp-be-lol-game-data/global/default/assets/items/icons2d/${id}.png`.toLowerCase(),
          price_total: 0,
        });
      }
    }
    for (const combo of buildEngineRec.active_combos ?? []) {
      for (const itemId of combo.item_ids ?? []) {
        if (!map.has(itemId)) {
          map.set(itemId, {
            id: itemId,
            name: `Item #${itemId}`,
            icon_url: `https://raw.communitydragon.org/latest/plugins/rcp-be-lol-game-data/global/default/assets/items/icons2d/${itemId}.png`.toLowerCase(),
            price_total: 0,
          });
        }
      }
    }
    return map;
  });

  const knownAugmentMap = $derived.by(() => {
    const map = new Map<string, MayhemBuildAugment>();
    for (const s of pickerSlots) {
      if (s) {
        map.set(s.id, {
          id: s.id,
          name: s.name,
          icon_url: s.icon_url,
          rarity: s.rarity || "Silver",
          tooltip: s.tooltip || "",
        });
        map.set(s.name.toLowerCase(), {
          id: s.id,
          name: s.name,
          icon_url: s.icon_url,
          rarity: s.rarity || "Silver",
          tooltip: s.tooltip || "",
        });
      }
    }
    return map;
  });

  $effect(() => {
    if (selectedChampion && selectedChampion.id > 0 && selectedChampion.id !== selectedChampionId) {
      selectedChampionId = selectedChampion.id;
    }
  });

  let lastRecKey = "";

  async function updateRecommendation(champId: number, itemIds: number[], augs: string[]) {
    if (!champId || champId <= 0) {
      buildEngineRec = null;
      lastRecKey = "";
      return;
    }
    const currentKey = `${champId}:${itemIds.slice().sort().join(",")}:${augs.slice().sort().join(",")}`;
    if (currentKey === lastRecKey) return;
    lastRecKey = currentKey;

    loadingRec = true;
    try {
      const rec = await computeMayhemBuildRecommendation(champId, itemIds, augs);
      buildEngineRec = rec;
      if (overlayEnabled) {
        void emitOverlayState({
          source: "main_window",
          championId: champId,
          championName: currentSelectedChampion?.name ?? selectedChampion?.name ?? "",
          strategyId: "engine_v2",
          augmentIds: augs,
          recommendedBoots: rec?.recommended_boots?.item_id,
          nextItemIds: rec?.next_items?.map((i) => i.item_id) ?? [],
          completedBuild: rec?.completed_build?.map((i) => (typeof i === "number" ? i : i.item_id)) ?? [],
          buildEngineRec: rec,
        });
      }
    } catch (err) {
      logger.warn("frontend.build_engine", "Failed computing recommendation", err);
    } finally {
      loadingRec = false;
    }
  }

  $effect(() => {
    const champId = selectedChampionId;
    const items = userBuildItemIds;
    const augs = augmentIds;
    void updateRecommendation(champId, items, augs);
  });

  function handleSelectChampion(champId: number) {
    selectedChampionId = champId;
    championPickerOpen = false;
    const champ = allChampions.find((c) => c.id === champId) ?? null;
    if (champ) {
      onSelectChampion?.(champ);
    }
  }

  async function handleSwapChampion(champId: number) {
    if (isInChampSelect) {
      try {
        await swapChampSelectChampion(champId);
      } catch (err) {
        logger.error("frontend.mayhem_champ_select", "Swap champion failed", err);
      }
      return;
    }
    handleSelectChampion(champId);
  }

  function toggleSimulatedAugment(augName: string) {
    if (augmentIds.includes(augName)) {
      augmentIds = augmentIds.filter((id) => id !== augName);
    } else {
      augmentIds = augmentIds.length >= 4 ? [...augmentIds.slice(1), augName] : [...augmentIds, augName];
    }
    const newSlots: (MayhemAugmentItem | null)[] = [null, null, null, null];
    augmentIds.forEach((id, idx) => {
      if (idx < 4) {
        newSlots[idx] = {
          id,
          name: id,
          icon_url: "",
          categories: [],
          games: 100,
          wins: 50,
          win_rate: 0.5,
          wpa: 0.05,
          global_games: 0,
          global_win_rate: 0,
          global_wpa: 0,
        };
      }
    });
    pickerSlots = newSlots;
  }

  function handleEquipItem(item: MayhemBuildItem) {
    if (selectedBuildSlotIndex !== null && selectedBuildSlotIndex >= 0 && selectedBuildSlotIndex < 6) {
      userBuildSlots[selectedBuildSlotIndex] = item;
      selectedBuildSlotIndex = null;
      return;
    }
    const emptyIdx = userBuildSlots.findIndex((s) => s === null);
    if (emptyIdx !== -1) {
      userBuildSlots[emptyIdx] = item;
    } else {
      userBuildSlots[5] = item;
    }
  }

  function handleClearSlot(idx: number) {
    userBuildSlots[idx] = null;
    if (selectedBuildSlotIndex === idx) {
      selectedBuildSlotIndex = null;
    }
  }

  onMount(async () => {
    try {
      allChampions = await getChampions();
    } catch (err) {
      logger.error("frontend.mayhem_champ_select", "Failed loading champions", err);
    }

    try {
      itemsCatalog = await getItemsCatalog();
    } catch (err) {
      logger.error("frontend.mayhem_champ_select", "Failed loading items catalog", err);
    }

    try {
      const sess = await getChampSelectSession();
      if (sess && sess.myTeam && sess.myTeam.length > 0) {
        session = sess;
        const myChamp = sess.myTeam.find((m) => m.cellId === sess.localPlayerCellId)?.championId;
        if (myChamp && myChamp > 0) {
          handleSelectChampion(myChamp);
        }
      }
    } catch {}

    try {
      const state = await getLiveGameState();
      if (state && state.is_in_game) {
        liveGameState = state;
      }
    } catch {}

    try {
      unlistenSession = await listen("lcu:champ-select-session", (event) => {
        const data = event.payload as ChampSelectSession | null;
        if (data && data.myTeam && data.myTeam.length > 0) {
          session = data;
          const myChamp = data.myTeam.find((m) => m.cellId === data.localPlayerCellId)?.championId;
          if (myChamp && myChamp > 0 && selectedChampionId !== myChamp) {
            handleSelectChampion(myChamp);
          }
        } else {
          session = null;
        }
      });

      unlistenPhase = await listen("lcu:gameflow-phase", async (event) => {
        const phase = event.payload as string;
        if (phase !== "ChampSelect") {
          session = null;
        } else {
          try {
            session = await getChampSelectSession();
          } catch {}
        }
      });

      unlistenLiveState = await listen("live:game-state", (event) => {
        const state = event.payload as LiveGameState;
        const wasInGame = Boolean(liveGameState?.is_in_game);
        liveGameState = state?.is_in_game ? state : null;
        if (state && state.is_in_game) {
          if (!wasInGame) {
            void syncOverlayWindow(overlayEnabled, false);
          }
          if (state.champion_name) {
            const matched = allChampions.find(
              (c) => c.name.toLowerCase() === state.champion_name.toLowerCase()
            );
            if (matched && matched.id !== selectedChampionId) {
              handleSelectChampion(matched.id);
            }
          }
          if (Array.isArray(state.items)) {
            const newSlots: (MayhemBuildItem | null)[] = Array(6).fill(null);
            const buildItems = state.items.filter((item) => {
              if (item.item_id === 2050 || item.item_id === 2052 || item.item_id === 220013) return false;
              if (item.display_name && item.display_name.toLowerCase().includes("poro-snax")) return false;
              const known = itemsCatalog[item.item_id];
              if (known?.name && known.name.toLowerCase().includes("poro-snax")) return false;
              return true;
            });

            buildItems.slice(0, 6).forEach((item, idx) => {
              const known = itemsCatalog[item.item_id];
              newSlots[idx] = {
                id: item.item_id,
                name: known?.name || `Item #${item.item_id}`,
                icon_url: known?.icon_url || `https://raw.communitydragon.org/latest/plugins/rcp-be-lol-game-data/global/default/assets/items/icons2d/${item.item_id}.png`.toLowerCase(),
                price_total: known?.price_total || 0,
                item_class: known?.item_class,
                description: known?.description,
              };
            });
            userBuildSlots = newSlots;
          }
        }
      });

      unlistenOverlaySync = await listen("overlay:state-sync", (event) => {
        const data = event.payload as { source?: string; augmentIds?: string[] };
        if (data.source === "main_window") return;
        if (Array.isArray(data.augmentIds)) {
          const incoming = data.augmentIds;
          const same = incoming.length === augmentIds.length && incoming.every((id, i) => id === augmentIds[i]);
          if (same) return;
          augmentIds = incoming;
          const newSlots: (MayhemAugmentItem | null)[] = [null, null, null, null];
          augmentIds.forEach((id, idx) => {
            if (idx < 4) {
              newSlots[idx] = {
                id,
                name: id,
                icon_url: "",
                categories: [],
                games: 100,
                wins: 50,
                win_rate: 0.5,
                wpa: 0.05,
                global_games: 0,
                global_win_rate: 0,
                global_wpa: 0,
              };
            }
          });
          pickerSlots = newSlots;
        }
      });
      void syncOverlayWindow(overlayEnabled, false);
    } catch (err) {
      logger.error("frontend.mayhem_champ_select", "Failed attaching event listeners", err);
    }
  });

  onDestroy(() => {
    unlistenSession?.();
    unlistenPhase?.();
    unlistenLiveState?.();
    unlistenOverlaySync?.();
  });
</script>

<div class="flex h-full min-h-0 flex-col bg-gray-950 text-gray-100 overflow-y-auto scrollbar-thin select-none relative">
  <!-- Scan Status Toast Banner -->
  {#if scanToast}
    <div class="absolute top-2 left-1/2 -translate-x-1/2 z-50 flex items-center gap-2 px-4 py-1.5 rounded-xl border text-xs font-semibold shadow-2xl backdrop-blur-md transition-all animate-bounce {scanToast.isError ? 'border-red-500/80 bg-red-950/95 text-red-200' : 'border-amber-500/80 bg-gray-950/95 text-amber-200'}">
      <span class={scanToast.isError ? "icon-[heroicons--exclamation-triangle] text-red-400 text-base" : "icon-[heroicons--information-circle] text-amber-400 text-base"}></span>
      <span>{scanToast.message}</span>
    </div>
  {/if}

  <div class="flex-1 p-5 space-y-4">
    <!-- Header: Match Status, Champ Select, Champion Picker, Overlay sync, Tools -->
    <div class="flex flex-wrap items-center justify-between gap-3 border-b border-gray-800/80 pb-3">
      <div class="flex items-center gap-3">
        <!-- Champion Selector Dropdown -->
        <div class="relative">
          <button
            type="button"
            onclick={() => (championPickerOpen = !championPickerOpen)}
            class="flex items-center gap-2.5 px-3 py-1.5 rounded-xl border border-gray-800 bg-gray-900/90 hover:border-gray-700 text-left transition cursor-pointer"
          >
            {#if currentSelectedChampion?.square_icon_url}
              <img
                src={currentSelectedChampion.square_icon_url}
                alt={currentSelectedChampion.name}
                class="h-7 w-7 rounded-lg object-cover border border-purple-500/50"
              />
            {:else}
              <div class="h-7 w-7 rounded-lg bg-gray-800 flex items-center justify-center text-gray-400">
                <span class="icon-[heroicons--user] text-sm"></span>
              </div>
            {/if}
            <div class="flex flex-col">
              <span class="text-xs font-bold text-gray-200 leading-tight">
                {currentSelectedChampion?.name || `Champ #${selectedChampionId}`}
              </span>
              <span class="text-[10px] text-gray-500">Click to switch</span>
            </div>
            <span class="icon-[heroicons--chevron-up-down] text-gray-400 text-xs ml-1"></span>
          </button>

          {#if championPickerOpen}
            <div class="absolute left-0 top-full mt-2 w-72 rounded-xl border border-gray-700 bg-gray-900 p-2 shadow-2xl z-50 space-y-2">
              <input
                type="text"
                placeholder="Search champion..."
                bind:value={championSearchQuery}
                class="w-full rounded-lg border border-gray-700 bg-gray-950 px-2.5 py-1 text-xs text-gray-200 placeholder-gray-500 focus:border-purple-500 focus:outline-none"
              />
              <div class="max-h-60 overflow-y-auto scrollbar-thin space-y-1">
                {#each filteredChampions as champ (champ.id)}
                  <button
                    type="button"
                    onclick={() => handleSelectChampion(champ.id)}
                    class="w-full flex items-center gap-2 px-2 py-1.5 rounded-lg text-left text-xs transition cursor-pointer {selectedChampionId === champ.id ? 'bg-purple-950/80 text-purple-200 border border-purple-500/40' : 'hover:bg-gray-800/80 text-gray-300'}"
                  >
                    <img src={champ.square_icon_url} alt={champ.name} class="h-6 w-6 rounded object-cover" />
                    <span class="truncate font-semibold">{champ.name}</span>
                  </button>
                {/each}
              </div>
            </div>
          {/if}
        </div>

        <!-- Live Match Active Badge -->
        {#if liveGameState?.is_in_game}
          <span class="inline-flex items-center gap-1.5 rounded-full border border-emerald-500/50 bg-emerald-950/80 px-2.5 py-1 text-[11px] font-bold text-emerald-200 animate-pulse">
            <span class="h-2 w-2 rounded-full bg-emerald-400"></span>
            Live Game: {liveGameState.champion_name} (Lv {liveGameState.level})
          </span>
        {/if}

        <!-- Champ Select Status Badge -->
        {#if isInChampSelect}
          <span class="inline-flex items-center gap-1.5 rounded-full border border-purple-500/50 bg-purple-950/80 px-2.5 py-1 text-[11px] font-bold text-purple-200 animate-pulse">
            <span class="h-2 w-2 rounded-full bg-purple-400"></span>
            Champ Select Active ({session?.myTeam?.length ?? 0}/5)
          </span>
        {:else if isLcuConnected}
          <span class="inline-flex items-center gap-1.5 rounded-full border border-gray-800 bg-gray-900/80 px-2.5 py-1 text-[11px] font-medium text-gray-400">
            <span class="h-1.5 w-1.5 rounded-full bg-gray-500"></span>
            Champ Select: Idle
          </span>
        {:else}
          <span class="inline-flex items-center gap-1.5 rounded-full border border-gray-800 bg-gray-900/40 px-2.5 py-1 text-[11px] font-medium text-gray-600">
            <span class="h-1.5 w-1.5 rounded-full bg-gray-600"></span>
            LCU Offline
          </span>
        {/if}
      </div>

      <!-- Action Buttons: Overlay sync, Scan cards, Encyclopedia -->
      <div class="flex items-center gap-2">
        <button
          type="button"
          onclick={() => {
            overlayEnabled = !overlayEnabled;
            void syncOverlayWindow(overlayEnabled && Boolean(liveGameState?.is_in_game), false);
          }}
          class="flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-semibold border transition cursor-pointer {overlayEnabled ? 'bg-purple-950/80 border-purple-500/50 text-purple-200' : 'bg-gray-900 border-gray-800 text-gray-400 hover:bg-gray-800'}"
          title="Toggle In-Game Mayhem Overlay"
        >
          <span class="icon-[heroicons--computer-desktop] text-sm {overlayEnabled ? 'text-purple-400' : 'text-gray-500'}"></span>
          Overlay: {overlayEnabled ? "ON" : "OFF"}
        </button>

        {#if overlayEnabled}
          <button
            type="button"
            onclick={async () => {
              void syncOverlayWindow(true, false);
              showScanToast("Scanning screen for augment cards...", false);
              try {
                const res = await scanLiveAugmentCards(selectedChampionId > 0 ? selectedChampionId : undefined, augmentIds);
                if (res && res.length > 0) {
                  showScanToast(`Successfully recognized ${res.length} augment cards`, false);
                } else {
                  showScanToast("No augment cards recognized on screen", true);
                }
              } catch (e) {
                const msg = typeof e === "string" ? e : (e as Error)?.message || "Scan failed";
                showScanToast(msg, true);
              }
            }}
            class="flex items-center gap-1 px-2.5 py-1.5 rounded-lg text-xs font-semibold bg-gray-900 hover:bg-gray-800 text-gray-300 border border-gray-800 transition cursor-pointer"
            title="Scan screen for Augment Cards (F8)"
          >
            <span class="icon-[heroicons--camera] text-purple-400 text-xs"></span>
            Scan Cards (F8)
          </button>
        {/if}

        <button
          type="button"
          onclick={() => (isEncyclopediaOpen = true)}
          class="flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-semibold bg-gray-900 hover:bg-gray-800 text-gray-200 border border-gray-800 transition cursor-pointer"
        >
          <span class="icon-[heroicons--book-open] text-purple-400 text-sm"></span>
          Encyclopedia
        </button>
      </div>
    </div>

    <!-- Section 1: Champ Select Roster Bar (if active) -->
    {#if isInChampSelect}
      <MayhemRosterBar
        {session}
        {allChampions}
        {selectedChampionId}
        onSelectChampion={handleSelectChampion}
        onSwapChampion={handleSwapChampion}
      />
    {/if}

    <!-- Section 2: Augment Simulation Bar -->
    <div class="rounded-xl border border-gray-800 bg-gray-900/60 p-3.5 space-y-3">
      <div class="flex flex-wrap items-center justify-between gap-2">
        <div class="flex items-center gap-2">
          <span class="icon-[heroicons--adjustments-horizontal] text-purple-400 text-base"></span>
          <span class="text-xs font-bold text-gray-200 uppercase tracking-wide">
            Augment Simulation & Build Warping
          </span>
          <span class="text-[11px] text-gray-400">Select or toggle augments to re-rank item paths</span>
        </div>
        {#if augmentIds.length > 0}
          <button
            type="button"
            onclick={() => {
              augmentIds = [];
              pickerSlots = [null, null, null, null];
            }}
            class="px-2 py-0.5 rounded text-[11px] font-semibold text-rose-400 hover:bg-rose-950/40 border border-rose-900/50 cursor-pointer"
          >
            Reset Augments
          </button>
        {/if}
      </div>

      <!-- Quick Warp Toggles -->
      <div class="flex flex-wrap items-center gap-1.5">
        {#each WARP_AUGMENTS as augName}
          {@const active = augmentIds.includes(augName)}
          <button
            type="button"
            onclick={() => toggleSimulatedAugment(augName)}
            class="px-2.5 py-1 rounded-lg text-xs font-semibold border transition flex items-center gap-1 cursor-pointer {active ? 'border-purple-400 bg-purple-950/90 text-purple-200 shadow-[0_0_8px_rgba(192,132,252,0.3)]' : 'border-gray-800 bg-gray-950/80 text-gray-400 hover:text-gray-200 hover:border-gray-700'}"
          >
            <span>{augName}</span>
            {#if active}
              <span class="icon-[heroicons--x-mark] text-[10px]"></span>
            {/if}
          </button>
        {/each}
      </div>

      <!-- 4-Slot Augment Picker -->
      <MayhemAugmentPicker
        championId={selectedChampionId}
        bind:slots={pickerSlots}
        onChange={(ids) => {
          augmentIds = ids;
        }}
      />
    </div>

    <!-- Section 3: Two-Column Main Content Grid -->
    <div class="grid grid-cols-1 xl:grid-cols-12 gap-4">
      <!-- Left Column: Recommendation Engine Panel + Build Simulator -->
      <div class="xl:col-span-7 space-y-4">
        <MayhemBuildEnginePanel
          recommendation={buildEngineRec}
          loading={loadingRec}
          {knownItemMap}
          {knownAugmentMap}
          onSelectItem={handleEquipItem}
        />

        <MayhemBuildSimulator
          buildSlots={userBuildSlots}
          selectedSlotIndex={selectedBuildSlotIndex}
          onSelectSlot={(idx) => {
            selectedBuildSlotIndex = selectedBuildSlotIndex === idx ? null : idx;
          }}
          onClearSlot={handleClearSlot}
          onReset={() => {
            userBuildSlots = Array(6).fill(null);
            selectedBuildSlotIndex = null;
          }}
        />
      </div>

      <!-- Right Column: Combos & Synergies Card -->
      <div class="xl:col-span-5 space-y-4">
        <MayhemCombosCard
          championId={selectedChampionId}
          championName={currentSelectedChampion?.name ?? ""}
          currentAugmentIds={augmentIds}
          currentBuildItemIds={userBuildItemIds}
          {hideLowPickrate}
          {agentOperations}
          {itemsCatalog}
          onSelectItem={(it) => {
            handleEquipItem({
              id: it.id,
              name: it.name,
              icon_url: it.icon_url,
              price_total: it.price_total ?? 0,
            });
          }}
          onSelectAugment={(aug) => {
            toggleSimulatedAugment(aug.name);
          }}
        />
      </div>
    </div>
  </div>

  <!-- Encyclopedia Modal -->
  <MayhemEncyclopediaModal
    isOpen={isEncyclopediaOpen}
    onClose={() => (isEncyclopediaOpen = false)}
    onSimulateAugment={(name) => toggleSimulatedAugment(name)}
  />
</div>
