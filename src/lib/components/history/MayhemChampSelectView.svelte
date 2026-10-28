<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import {
    getChampSelectSession,
    getMayhemChampionBuild,
    getMayhemTeamBuilds,
    exportMayhemBuild,
    saveMayhemBuild,
    swapChampSelectChampion,
    getLiveGameState,
    getLiveBuildAdvice,
  } from "../../api/champ_select";
  import { getChampions } from "../../api/champions";
  import type { ChampionSummary } from "../../types/champions";
  import type {
    ChampSelectSession,
    MayhemAugmentItem,
    MayhemChampionBuildSummary,
    MayhemBuildStrategy,
    MayhemBuildAugment,
    LiveGameState,
    MayhemLiveAdvice,
  } from "../../types/champ_select";
  import AugmentIcon from "../common/AugmentIcon.svelte";
  import GameEntityPopover from "../common/GameEntityPopover.svelte";
  import MayhemAugmentsCard from "./MayhemAugmentsCard.svelte";
  import MayhemAugmentPicker from "./MayhemAugmentPicker.svelte";
  import LivePurchaseAdvisor from "./LivePurchaseAdvisor.svelte";
  import StagePathingTree from "./StagePathingTree.svelte";
  import MayhemEncyclopediaModal from "./MayhemEncyclopediaModal.svelte";
  import { logger } from "../../utils/logger";

  let {
    selectedChampion = null,
    onSelectChampion,
    hideLowPickrate = false,
    isLcuConnected = false,
  }: {
    selectedChampion?: ChampionSummary | null;
    onSelectChampion?: (champ: ChampionSummary | null) => void;
    hideLowPickrate?: boolean;
    isLcuConnected?: boolean;
  } = $props();

  let session = $state<ChampSelectSession | null>(null);
  let teamBuilds = $state<Record<number, MayhemChampionBuildSummary>>({});
  let allChampions = $state<ChampionSummary[]>([]);
  let selectedChampionId = $state<number>(22);

  $effect(() => {
    if (selectedChampion && selectedChampion.id > 0 && selectedChampion.id !== selectedChampionId) {
      selectedChampionId = selectedChampion.id;
      if (!teamBuilds[selectedChampion.id]) {
        void loadSingleChampionBuild(selectedChampion.id);
      }
    }
  });
  let selectedStrategyId = $state<string>("archetype_1");
  let gameflowPhase = $state<string>("");
  let pickerSlots = $state<(MayhemAugmentItem | null)[]>([null, null, null, null]);
  let augmentIds = $state<string[]>([]);
  let savePrompt = $state<{ strategy: MayhemBuildStrategy; build: MayhemChampionBuildSummary; augmentIds: string[] } | null>(null);
  const isInProgress = $derived(gameflowPhase === "InProgress");
  let manualChampionBuild = $state<MayhemChampionBuildSummary | null>(null);
  let loadingBuild = $state(false);
  let strategyDropdownOpen = $state(false);
  let exporting = $state(false);
  let exportToast = $state<{ message: string; success: boolean } | null>(null);
  let hoverTooltip = $state<{
    visible: boolean;
    x: number;
    y: number;
    summary: MayhemChampionBuildSummary | null;
  }>({ visible: false, x: 0, y: 0, summary: null });

  let unlistenSession: UnlistenFn | null = null;
  let unlistenPhase: UnlistenFn | null = null;
  let unlistenLiveState: UnlistenFn | null = null;
  let liveGameState = $state<LiveGameState | null>(null);
  let liveAdvice = $state<MayhemLiveAdvice | null>(null);
  let isEncyclopediaOpen = $state(false);
  let livePollTimer: ReturnType<typeof setInterval> | null = null;
  let buildAlgorithmVersion = $state<"v1" | "v2">(
    (typeof localStorage !== "undefined" && (localStorage.getItem("mayhem-build-algorithm-version") as "v1" | "v2")) || "v2"
  );

  function setBuildAlgorithmVersion(v: "v1" | "v2") {
    if (buildAlgorithmVersion === v) return;
    buildAlgorithmVersion = v;
    try {
      localStorage.setItem("mayhem-build-algorithm-version", v);
    } catch (_) {}
    if (selectedChampionId > 0) {
      void loadSingleChampionBuild(selectedChampionId);
    }
  }

  const isInChampSelect = $derived(
    Boolean(
      session &&
        session.myTeam &&
        session.myTeam.length > 0 &&
        (session.queueId === 2400 || session.benchEnabled)
    )
  );

  const availableRosterChampionIds = $derived.by<Set<number>>(() => {
    const ids = new Set<number>();
    if (session?.benchChampions) {
      for (const b of session.benchChampions) {
        if (b.championId > 0) ids.add(b.championId);
      }
    }
    if (session?.myTeam) {
      for (const m of session.myTeam) {
        if (m.championId > 0 && m.cellId !== session.localPlayerCellId) {
          ids.add(m.championId);
        }
      }
    }
    return ids;
  });

  const inspectedBuild = $derived<MayhemChampionBuildSummary | null>(
    teamBuilds[selectedChampionId] ?? manualChampionBuild
  );

  const availableStrategies = $derived<MayhemBuildStrategy[]>(
    inspectedBuild?.strategies ?? []
  );

  const activeStrategy = $derived<MayhemBuildStrategy | null>(
    availableStrategies.find((s) => s.id === selectedStrategyId) ??
      availableStrategies[0] ??
      null
  );

  const otherSpells = $derived(
    activeStrategy?.all_spells
      ? activeStrategy.all_spells.filter(
          (s) => !activeStrategy.spells.some((cs) => cs.id === s.id)
        )
      : []
  );
  const sortedSituationalItems = $derived(
    activeStrategy?.situational_items
      ? [...activeStrategy.situational_items].sort(
          (a, b) => (b.games ?? 0) - (a.games ?? 0) || (b.win_rate ?? 0) - (a.win_rate ?? 0)
        )
      : []
  );


  const currentSelectedChampion = $derived(
    allChampions.find((c) => c.id === selectedChampionId) ?? null
  );

  onMount(async () => {
    try {
      allChampions = await getChampions();
    } catch (err) {
      logger.error("frontend.mayhem_champ_select", "Failed loading champions", err);
    }

    // Check initial active session and connection
    await refreshSession();
    await refreshLiveGameState();
    livePollTimer = setInterval(refreshLiveGameState, 2500);

    // Listen to real-time events
    try {
      unlistenSession = await listen("lcu:champ-select-session", async (event) => {
        const data = event.payload as ChampSelectSession | null;
        if (data && data.myTeam && data.myTeam.length > 0) {
          session = data;
          await handleSessionUpdated(data);
        } else {
          session = null;
        }
      });

      unlistenPhase = await listen("lcu:gameflow-phase", async (event) => {
        const phase = event.payload as string;
        handlePhaseChange(phase);
        if (phase !== "ChampSelect") {
          session = null;
        } else {
          await refreshSession();
        }
      });

      unlistenLiveState = await listen("live:game-state", (event) => {
        const state = event.payload as LiveGameState;
        liveGameState = state;
        if (state && state.is_in_game) {
          void refreshLiveGameState();
        } else {
          liveAdvice = null;
        }
      });
    } catch (err) {
      logger.error("frontend.mayhem_champ_select", "Failed attaching event listeners", err);
    }

    // If not in champ select, load default champion build and augments
    if (!isInChampSelect && selectedChampionId > 0) {
      await loadSingleChampionBuild(selectedChampionId);
    }
  });

  onDestroy(() => {
    if (unlistenSession) unlistenSession();
    if (unlistenPhase) unlistenPhase();
    if (unlistenLiveState) unlistenLiveState();
    if (livePollTimer) clearInterval(livePollTimer);
  });

  async function refreshSession() {
    try {
      const activeSession = await getChampSelectSession();
      if (activeSession && activeSession.myTeam && activeSession.myTeam.length > 0) {
        session = activeSession;
        await handleSessionUpdated(activeSession);
      } else {
        session = null;
      }
    } catch (err) {
      session = null;
      logger.warn("frontend.mayhem_champ_select", "Champ select session unavailable", err);
    }
  }

  async function refreshLiveGameState() {
    try {
      const state = await getLiveGameState();
      liveGameState = state;
      if (state && state.is_in_game && state.champion_name) {
        const matched = allChampions.find(
          (c) => c.name.toLowerCase() === state.champion_name.toLowerCase()
        );
        if (matched && matched.id !== selectedChampionId) {
          selectedChampionId = matched.id;
          void loadSingleChampionBuild(matched.id);
        }

        const activeChampId = matched ? matched.id : selectedChampionId;
        if (activeChampId > 0 && activeStrategy) {
          const inventoryIds = state.items.map((i) => i.item_id);
          try {
            liveAdvice = await getLiveBuildAdvice({
              championId: activeChampId,
              strategyId: selectedStrategyId,
              currentGold: state.current_gold,
              inventoryItemIds: inventoryIds,
              level: state.level,
              gameTime: state.game_time,
              augmentIds,
            });
          } catch (err) {
            logger.warn("frontend.live_advice", "Failed computing live advice", err);
          }
        }
      } else {
        liveAdvice = null;
      }
    } catch {
      liveGameState = null;
      liveAdvice = null;
    }
  }

  function toggleSimulatedAugment(augName: string) {
    if (augmentIds.includes(augName)) {
      augmentIds = augmentIds.filter((id) => id !== augName);
    } else {
      if (augmentIds.length >= 4) {
        augmentIds = [...augmentIds.slice(1), augName];
      } else {
        augmentIds = [...augmentIds, augName];
      }
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
    void loadSingleChampionBuild(selectedChampionId);
  }

  async function handleSessionUpdated(sess: ChampSelectSession) {
    const ids: number[] = [];
    if (sess.myTeam) {
      for (const m of sess.myTeam) {
        if (m.championId > 0 && !ids.includes(m.championId)) {
          ids.push(m.championId);
        }
      }
    }
    if (sess.benchChampions) {
      for (const b of sess.benchChampions) {
        if (b.championId > 0 && !ids.includes(b.championId)) {
          ids.push(b.championId);
        }
      }
    }

    if (ids.length > 0) {
      try {
        const builds = await getMayhemTeamBuilds(ids, buildAlgorithmVersion);
        teamBuilds = { ...teamBuilds, ...builds };
      } catch (err) {
        logger.error("frontend.mayhem_champ_select", "Failed batch loading team builds", err);
      }
    }

    // Auto-select local player's champion if picked
    const myChampId = sess.myTeam?.find(
      (m) => m.cellId === sess.localPlayerCellId
    )?.championId;

    if (myChampId && myChampId > 0) {
      if (selectedChampionId !== myChampId) {
        selectedChampionId = myChampId;
        const matched = allChampions.find((c) => c.id === myChampId) ?? null;
        if (matched) {
          onSelectChampion?.(matched);
        }
        if (!teamBuilds[myChampId]) {
          void loadSingleChampionBuild(myChampId);
        }
      }
    } else if (ids.length > 0 && (!selectedChampionId || selectedChampionId === 0)) {
      selectedChampionId = ids[0];
    }
  }

  async function handleRosterChampionClick(champId: number) {
    if (isInChampSelect) {
      // In champ select: can only change pick through client, or swap with available roster (bench/teammate)
      if (availableRosterChampionIds.has(champId)) {
        try {
          await swapChampSelectChampion(champId);
          // Actual state in UI is ONLY updated through LCU-sent messages we receive!
        } catch (err) {
          logger.error("frontend.mayhem_champ_select", "Failed requesting champ swap from LCU", err);
        }
      }
      return;
    }

    handleSelectChampion(champId);
  }

  async function loadSingleChampionBuild(championId: number) {
    if (!championId || championId <= 0) return;
    loadingBuild = true;
    try {
      const build = await getMayhemChampionBuild(championId, augmentIds, buildAlgorithmVersion);
      manualChampionBuild = build;
      teamBuilds[championId] = build;
      if (build.strategies.length > 0 && !build.strategies.some((s) => s.id === selectedStrategyId)) {
        selectedStrategyId = build.strategies[0].id;
      }
    } catch (err) {
      logger.error("frontend.mayhem_champ_select", "Failed loading champion build", err);
    } finally {
      loadingBuild = false;
    }
  }

  function handlePhaseChange(phase: string) {
    const prev = gameflowPhase;
    gameflowPhase = phase;
    const ended = phase === "PreEndOfGame" || phase === "EndOfGame" || phase === "WaitingForStats";
    if (prev === "InProgress" && ended && activeStrategy && inspectedBuild) {
      savePrompt = { strategy: activeStrategy, build: inspectedBuild, augmentIds: [...augmentIds] };
    }
    if (phase === "Lobby" || phase === "None" || phase === "ChampSelect") {
      pickerSlots = [null, null, null, null];
      if (augmentIds.length > 0) {
        augmentIds = [];
        void loadSingleChampionBuild(selectedChampionId);
      }
    }
  }

  function handleAugmentsChanged(ids: string[]) {
    augmentIds = ids;
    void loadSingleChampionBuild(selectedChampionId);
  }

  function handleSelectBuildAugment(aug: MayhemBuildAugment) {
    if (pickerSlots.some((s) => s?.id === aug.id)) return;
    const nextEmptyIdx = pickerSlots.findIndex((s) => s === null);
    if (nextEmptyIdx !== -1) {
      const item: MayhemAugmentItem = {
        id: aug.id,
        name: aug.name,
        icon_url: aug.icon_url,
        rarity: aug.rarity,
        tooltip: aug.tooltip,
        categories: [],
        games: aug.games ?? 0,
        wins: 0,
        win_rate: aug.win_rate ?? 0.5,
        wpa: aug.wpa ?? 0,
        pick_rate: aug.pick_rate ?? 0,
        global_games: 0,
        global_win_rate: 0,
        global_wpa: 0,
        global_pick_rate: 0,
      };
      pickerSlots[nextEmptyIdx] = item;
      const ids = pickerSlots.filter((s): s is MayhemAugmentItem => s !== null).map((s) => s.id);
      handleAugmentsChanged(ids);
    }
  }

  async function handleSaveBuild() {
    if (!savePrompt) return;
    const { strategy, build, augmentIds: ids } = savePrompt;
    try {
      await saveMayhemBuild({
        champion_id: build.champion_id,
        champion_name: build.champion_name,
        strategy_label: strategy.label,
        augment_ids: ids,
        build: strategy,
      });
      exportToast = { message: "Build saved", success: true };
    } catch (err) {
      exportToast = { message: `Save error: ${String(err)}`, success: false };
    }
    savePrompt = null;
    setTimeout(() => {
      exportToast = null;
    }, 5000);
  }

  function handleSelectChampion(champId: number) {
    selectedChampionId = champId;
    const champObj = allChampions.find((c) => c.id === champId) ?? null;
    if (champObj) {
      onSelectChampion?.(champObj);
    }
    if (!teamBuilds[champId]) {
      void loadSingleChampionBuild(champId);
    } else {
      const build = teamBuilds[champId];
      if (build?.strategies?.length > 0 && !build.strategies.some((s) => s.id === selectedStrategyId)) {
        selectedStrategyId = build.strategies[0].id;
      }
    }
  }

  function handleMouseEnterCard(event: MouseEvent, champId: number) {
    const summary = teamBuilds[champId] ?? null;
    if (!summary) return;
    const rect = (event.currentTarget as HTMLElement).getBoundingClientRect();
    hoverTooltip = {
      visible: true,
      x: rect.left + rect.width / 2,
      y: rect.bottom + 8,
      summary,
    };
  }

  function handleMouseLeaveCard() {
    hoverTooltip = { visible: false, x: 0, y: 0, summary: null };
  }

  async function handleExportBuild() {
    if (!inspectedBuild || !activeStrategy) return;
    exporting = true;
    exportToast = null;
    try {
      const itemIds = activeStrategy.items.map((i) => i.id);
      const starterId = activeStrategy.starter_item?.id ?? null;
      const spell1 = activeStrategy.spells[0]?.id ?? 4;
      const spell2 = activeStrategy.spells[1]?.id ?? 32;

      const res = await exportMayhemBuild({
        champion_id: inspectedBuild.champion_id,
        champion_name: inspectedBuild.champion_name,
        strategy_name: activeStrategy.label,
        starter_item_id: starterId,
        item_ids: itemIds,
        spell1_id: spell1,
        spell2_id: spell2,
      });

      exportToast = { message: res.message, success: res.success };
      setTimeout(() => {
        exportToast = null;
      }, 5000);
    } catch (err) {
      exportToast = { message: `Export error: ${String(err)}`, success: false };
      setTimeout(() => {
        exportToast = null;
      }, 5000);
    } finally {
      exporting = false;
    }
  }

  function formatPct(val: number): string {
    return `${(val * 100).toFixed(1)}%`;
  }

  function formatWpa(val: number): string {
    const sign = val >= 0 ? "+" : "";
    return `${sign}${(val * 100).toFixed(1)}%`;
  }
</script>

<div class="flex h-full min-h-0 flex-col bg-gray-950 text-gray-100 overflow-y-auto scrollbar-thin select-none relative">
  <div class="flex-1 p-6 space-y-6">
    <!-- Minimalistic Connection & Champ Select Status Header (Synchronized X-axis with content) -->
    <div class="flex flex-wrap items-center justify-between gap-3">
      <div class="flex items-center gap-2">
        <span class="text-sm font-bold text-gray-200 tracking-wide">ARAM Mayhem</span>
        <span class="text-gray-600">/</span>
        <span class="text-xs text-gray-400">Champion Selection</span>
      </div>

      <!-- Status Indicators -->
      <div class="flex items-center gap-2">
        <!-- V1 / V2 Build Algorithm Toggle -->
        <div class="flex items-center rounded-lg border border-gray-700/60 bg-gray-850 p-0.5 text-xs font-semibold shadow-sm">
          <button
            type="button"
            onclick={() => setBuildAlgorithmVersion("v1")}
            class="px-2 py-0.5 rounded transition text-[11px] {buildAlgorithmVersion === 'v1' ? 'bg-purple-600 text-white font-bold shadow' : 'text-gray-400 hover:text-gray-200'}"
            title="Algorithm V1: Classic Metric & Archetype Mining"
          >
            V1
          </button>
          <button
            type="button"
            onclick={() => setBuildAlgorithmVersion("v2")}
            class="px-2 py-0.5 rounded transition text-[11px] {buildAlgorithmVersion === 'v2' ? 'bg-purple-600 text-white font-bold shadow' : 'text-gray-400 hover:text-gray-200'}"
            title="Algorithm V2: Purchase Priority Inference & Adaptive Augment Synergy"
          >
            V2
          </button>
        </div>

        <!-- Encyclopedia Button -->
        <button
          type="button"
          onclick={() => isEncyclopediaOpen = true}
          class="flex items-center gap-1.5 px-3 py-1 rounded-lg text-xs font-semibold bg-gray-800/80 hover:bg-gray-700 text-gray-200 border border-gray-700/60 transition shadow-sm cursor-pointer"
        >
          <span class="icon-[heroicons--book-open] text-purple-400 text-sm"></span>
          Encyclopedia
        </button>

        <!-- Live Match Active Badge -->
        {#if liveGameState?.is_in_game}
          <span class="inline-flex items-center gap-1.5 rounded-full border border-emerald-500/50 bg-emerald-950/80 px-2.5 py-0.5 text-[11px] font-bold text-emerald-200 shadow-sm animate-pulse">
            <span class="h-1.5 w-1.5 rounded-full bg-emerald-400"></span>
            Match: {liveGameState.champion_name} (Lv {liveGameState.level})
          </span>
        {/if}

        <!-- Champ Select Status Indicator -->
        {#if isInChampSelect}
          <span class="inline-flex items-center gap-1.5 rounded-full border border-purple-500/40 bg-purple-950/70 px-2.5 py-0.5 text-[11px] font-semibold text-purple-200 animate-pulse">
            <span class="h-1.5 w-1.5 rounded-full bg-purple-400"></span>
            Champ Select: Active ({session?.myTeam?.length ?? 0}/5)
          </span>
        {:else if isLcuConnected}
          <span class="inline-flex items-center gap-1.5 rounded-full border border-gray-700/80 bg-gray-800/80 px-2.5 py-0.5 text-[11px] font-medium text-gray-400">
            <span class="h-1.5 w-1.5 rounded-full bg-gray-500"></span>
            Champ Select: Idle
          </span>
        {:else}
          <span class="inline-flex items-center gap-1.5 rounded-full border border-gray-800 bg-gray-950 px-2.5 py-0.5 text-[11px] font-medium text-gray-500">
            <span class="h-1.5 w-1.5 rounded-full bg-gray-600"></span>
            Champ Select: Offline
          </span>
        {/if}
      </div>
    </div>

    <!-- Live In-Game Purchase Advisor -->
    {#if liveGameState?.is_in_game}
      <LivePurchaseAdvisor {liveGameState} {liveAdvice} {activeStrategy} />
    {/if}

    <!-- Augment Simulation Quick-Toggles -->
    <div class="rounded-xl border border-gray-800 bg-gray-900/40 p-3 flex flex-wrap items-center justify-between gap-3">
      <div class="flex items-center gap-2">
        <span class="text-xs font-bold text-gray-300 uppercase tracking-wide flex items-center gap-1.5">
          <span class="icon-[heroicons--adjustments-horizontal] text-purple-400 text-sm"></span>
          Augment Build Warping Simulator:
        </span>
        <span class="text-[11px] text-gray-400">Click to preview adaptive build paths</span>
      </div>

      <div class="flex flex-wrap items-center gap-1.5">
        {#each ["Marksmage", "ADAPt", "Big Shot", "Apex Inventor", "Vulnerability", "Goliath", "Draw Your Sword"] as augName}
          {@const isSimulated = augmentIds.includes(augName)}
          <button
            type="button"
            onclick={() => toggleSimulatedAugment(augName)}
            class="px-2.5 py-1 rounded-lg text-xs font-semibold transition border flex items-center gap-1 cursor-pointer {isSimulated
              ? 'border-purple-400 bg-purple-950/80 text-purple-200 shadow-[0_0_8px_rgba(192,132,252,0.3)]'
              : 'border-gray-800 bg-gray-950 text-gray-400 hover:text-gray-200 hover:border-gray-700'}"
          >
            <span>{augName}</span>
            {#if isSimulated}
              <span class="icon-[heroicons--x-mark] text-[10px]"></span>
            {/if}
          </button>
        {/each}

        {#if augmentIds.length > 0}
          <button
            type="button"
            onclick={() => { augmentIds = []; pickerSlots = [null, null, null, null]; void loadSingleChampionBuild(selectedChampionId); }}
            class="px-2 py-1 rounded-lg text-[11px] font-semibold text-rose-400 hover:bg-rose-950/30 transition border border-rose-900/50 cursor-pointer"
          >
            Reset
          </button>
        {/if}
      </div>
    </div>
    <!-- Section 1: Team & Bench Rosters (Visible in Champ Select) -->
    {#if isInChampSelect}
      <div class="grid grid-cols-1 lg:grid-cols-2 gap-4">
        <!-- Allied Team Roster -->
        <div class="rounded-xl border border-gray-800 bg-gray-900/40 p-4">
          <div class="flex items-center justify-between mb-3">
            <h2 class="text-xs font-semibold uppercase tracking-wider text-gray-400 flex items-center gap-1.5">
              <span class="icon-[heroicons--user-group] h-4 w-4 text-purple-400"></span>
              Allied Team ({session?.myTeam?.length ?? 0}/5)
            </h2>
            <span class="text-[11px] text-gray-500">Hover for build · Click to inspect</span>
          </div>
          <div class="grid grid-cols-5 gap-2.5">
            {#each session?.myTeam ?? [] as member (member.cellId)}
              {@const isMe = member.cellId === session?.localPlayerCellId}
              {@const champSummary = teamBuilds[member.championId]}
              {@const champCatalog = allChampions.find((c) => c.id === member.championId)}
              {@const isInspected = selectedChampionId === member.championId}
              <button
                type="button"
                title={isMe ? "Your current pick" : "Click to request swap from teammate"}
                onclick={() => member.championId > 0 && (!isMe ? handleRosterChampionClick(member.championId) : handleSelectChampion(member.championId))}
                onmouseenter={(e) => member.championId > 0 && handleMouseEnterCard(e, member.championId)}
                onmouseleave={handleMouseLeaveCard}
                class="relative flex flex-col items-center justify-center p-2 rounded-xl border transition-all cursor-pointer {isInspected
                  ? 'border-purple-500 bg-purple-950/30 shadow-[0_0_12px_rgba(168,85,247,0.3)]'
                  : isMe
                  ? 'border-cyan-500/70 bg-cyan-950/20'
                  : 'border-gray-800 bg-gray-900/60 hover:border-gray-700'}"
              >
                {#if isMe}
                  <span class="absolute -top-2 left-1/2 -translate-x-1/2 rounded bg-cyan-600 px-1 py-0.2 text-[8px] font-bold uppercase tracking-wider text-white">
                    YOU
                  </span>
                {/if}
                <div class="relative h-12 w-12 rounded-lg overflow-hidden border border-gray-700 bg-gray-950 mb-1.5">
                  {#if champCatalog?.square_icon_url || champSummary?.icon_url}
                    <img
                      src={champCatalog?.square_icon_url || champSummary?.icon_url}
                      alt="Champion"
                      class="h-full w-full object-cover"
                    />
                  {:else}
                    <div class="flex h-full w-full items-center justify-center text-gray-600">
                      <span class="icon-[heroicons--user] h-6 w-6"></span>
                    </div>
                  {/if}
                </div>
                <span class="text-[11px] font-semibold text-gray-200 truncate max-w-full">
                  {champCatalog?.name || champSummary?.champion_name || "Picking..."}
                </span>
                {#if champSummary}
                  <div class="flex items-center gap-1 mt-0.5 text-[10px]">
                    <span class="font-bold text-gray-300">{formatPct(champSummary.champion_win_rate)}</span>
                    <span class="font-semibold {champSummary.champion_wpa >= 0 ? 'text-emerald-400' : 'text-rose-400'}">
                      {formatWpa(champSummary.champion_wpa)}
                    </span>
                  </div>
                {:else if member.championId > 0}
                  <span class="text-[10px] text-gray-500 mt-0.5">Mining...</span>
                {/if}
              </button>
            {/each}
          </div>
        </div>

        <!-- Bench / Available Champions -->
        <div class="rounded-xl border border-gray-800 bg-gray-900/40 p-4">
          <div class="flex items-center justify-between mb-3">
            <h2 class="text-xs font-semibold uppercase tracking-wider text-gray-400 flex items-center gap-1.5">
              <span class="icon-[heroicons--arrow-path-rounded-square] h-4 w-4 text-cyan-400"></span>
              Available Bench ({session?.benchChampions?.length ?? 0})
            </h2>
            <span class="text-[11px] text-gray-500">Hover for build · Click to inspect</span>
          </div>
          {#if (session?.benchChampions?.length ?? 0) === 0}
            <div class="flex h-24 items-center justify-center rounded-lg border border-dashed border-gray-800 text-xs text-gray-500">
              No champions currently on the bench.
            </div>
          {:else}
            <div class="flex flex-wrap gap-2">
              {#each session?.benchChampions ?? [] as bench (bench.championId)}
                {@const champSummary = teamBuilds[bench.championId]}
                {@const champCatalog = allChampions.find((c) => c.id === bench.championId)}
                {@const isInspected = selectedChampionId === bench.championId}
                <button
                  type="button"
                  title="Click to swap this champion from bench"
                  onclick={() => handleRosterChampionClick(bench.championId)}
                  onmouseenter={(e) => handleMouseEnterCard(e, bench.championId)}
                  onmouseleave={handleMouseLeaveCard}
                  class="flex items-center gap-2 p-1.5 pr-2.5 rounded-lg border transition-all cursor-pointer {isInspected
                    ? 'border-purple-500 bg-purple-950/40 shadow-sm'
                    : 'border-gray-800 bg-gray-900/60 hover:border-gray-700'}"
                >
                  <img
                    src={champCatalog?.square_icon_url || champSummary?.icon_url}
                    alt="Bench"
                    class="h-7 w-7 rounded-md object-cover bg-gray-950 border border-gray-700"
                  />
                  <div class="flex flex-col text-left">
                    <span class="text-[11px] font-semibold text-gray-200 leading-tight">
                      {champCatalog?.name || champSummary?.champion_name || `Champ #${bench.championId}`}
                    </span>
                    {#if champSummary}
                      <span class="text-[10px] font-bold {champSummary.champion_wpa >= 0 ? 'text-emerald-400' : 'text-rose-400'}">
                        {formatPct(champSummary.champion_win_rate)} ({formatWpa(champSummary.champion_wpa)})
                      </span>
                    {/if}
                  </div>
                </button>
              {/each}
            </div>
          {/if}
        </div>
      </div>
    {/if}

    <!-- In-game augment picker (levels 3/7/11/15) -->
    {#if isInProgress && selectedChampionId > 0}
      <MayhemAugmentPicker
        championId={selectedChampionId}
        bind:slots={pickerSlots}
        fallbackUsed={inspectedBuild?.fallback_used ?? false}
        onChange={handleAugmentsChanged}
      />
    {/if}

    {#if savePrompt}
      <div class="flex items-center justify-between gap-3 rounded-xl border border-emerald-800/60 bg-emerald-950/40 px-4 py-2.5 text-xs text-emerald-200">
        <span>Match ended. Save <strong>{savePrompt.strategy.label}</strong> for {savePrompt.build.champion_name}?</span>
        <div class="flex gap-2">
          <button type="button" onclick={handleSaveBuild} class="cursor-pointer rounded-lg bg-emerald-600 px-3 py-1 font-semibold text-white hover:bg-emerald-500">Save</button>
          <button type="button" onclick={() => (savePrompt = null)} class="cursor-pointer rounded-lg border border-gray-700 px-3 py-1 text-gray-300 hover:bg-gray-800">Dismiss</button>
        </div>
      </div>
    {/if}


    <!-- Main Champion Card alongside Dedicated Complete Augments Card -->
    <div class="grid grid-cols-1 xl:grid-cols-12 gap-6 items-start">
      <!-- Left Column: Main Champion Card -->
      <div class="xl:col-span-7 space-y-6 min-w-0">

    <!-- Section 2: Detailed Inspected Champion & Strategy Analysis -->
    {#if inspectedBuild}
      <div class="rounded-2xl border border-gray-800 bg-gray-900/60 p-6 shadow-xl space-y-6">
        <!-- Champion Hero Banner & Strategy Controls -->
        <div class="flex flex-wrap items-center justify-between gap-4 border-b border-gray-800 pb-5">
          <!-- Champion Info -->
          <div class="flex items-center gap-4">
            <div class="relative h-16 w-16 rounded-xl overflow-hidden border-2 border-purple-500/60 bg-gray-950 shadow-md">
              <img src={inspectedBuild.icon_url} alt={inspectedBuild.champion_name} class="h-full w-full object-cover" />
            </div>
            <div>
              <div class="flex items-center gap-2">
                <h2 class="text-xl font-bold text-gray-100">{inspectedBuild.champion_name}</h2>
                <span class="rounded bg-purple-950 border border-purple-800 px-2 py-0.5 text-xs font-mono font-semibold text-purple-300">
                  ARAM Mayhem
                </span>
              </div>
              <div class="flex items-center gap-3 mt-1 text-xs text-gray-400">
                <span>Base WR: <strong class="text-gray-200">{formatPct(inspectedBuild.champion_win_rate)}</strong></span>
                <span>•</span>
                <span>WPA: <strong class="{inspectedBuild.champion_wpa >= 0 ? 'text-emerald-400' : 'text-rose-400'}">{formatWpa(inspectedBuild.champion_wpa)}</strong></span>
                <span>•</span>
                <span>{inspectedBuild.total_games.toLocaleString()} games sampled</span>
              </div>
            </div>
          </div>

          <!-- Strategy Selector & Export Inputs (Right of Champion Info) -->
          <div class="flex flex-wrap items-start gap-3">
            <!-- Strategy Selector Column -->
            <div class="flex flex-col">
              <div class="flex items-center justify-between gap-2 mb-1">
                <span class="text-[10px] font-semibold uppercase tracking-wider text-gray-400">
                  Strategy
                </span>
                {#if activeStrategy}
                  <div class="flex items-center gap-1.5 text-[10px] font-mono">
                    <span class="rounded px-1.5 py-0.5 font-bold {activeStrategy.build_wpa >= 0 ? 'bg-emerald-950 text-emerald-400 border border-emerald-800/60' : 'bg-rose-950 text-rose-400 border border-rose-800/60'}">
                      {formatWpa(activeStrategy.build_wpa)} WPA
                    </span>
                    <span class="rounded bg-gray-800/80 px-1.5 py-0.5 text-gray-400">
                      {activeStrategy.sample_games.toLocaleString()} games
                    </span>
                  </div>
                {/if}
              </div>
              <div class="relative">
                <button
                  type="button"
                  onclick={() => (strategyDropdownOpen = !strategyDropdownOpen)}
                  class="w-64 sm:w-72 flex items-center justify-between gap-2 rounded-xl border border-gray-700 bg-gray-900 px-3.5 py-2 text-xs font-medium text-gray-200 transition-colors hover:border-purple-500 focus:outline-none cursor-pointer"
                >
                  <span class="font-bold text-white truncate min-w-0 flex-1 text-left">
                    {activeStrategy?.label ?? "Select Strategy"}
                  </span>
                  <span class="icon-[heroicons--chevron-down] h-4 w-4 text-gray-400 shrink-0"></span>
                </button>

                {#if strategyDropdownOpen}
                  <div class="absolute right-0 top-11 z-40 w-96 rounded-xl border border-gray-700 bg-gray-900 shadow-2xl p-1.5 space-y-1">
                    <div class="px-2.5 py-1 text-[10px] font-semibold uppercase tracking-wider text-gray-400">
                      Optimized Build Strategies (Sorted by highest WPA)
                    </div>
                    {#each availableStrategies as strat (strat.id)}
                      {@const isSelected = activeStrategy?.id === strat.id}
                      <button
                        type="button"
                        onclick={() => {
                          selectedStrategyId = strat.id;
                          strategyDropdownOpen = false;
                        }}
                        class="flex w-full items-center justify-between rounded-lg px-3 py-2 text-left text-xs transition-colors cursor-pointer {isSelected
                          ? 'bg-purple-600/30 text-white border border-purple-500/50'
                          : 'text-gray-300 hover:bg-gray-800 hover:text-white'}"
                      >
                        <div class="flex flex-col min-w-0">
                          <span class="font-semibold truncate">{strat.label}</span>
                          <span class="text-[10px] text-gray-400">{strat.avg_metric_value}</span>
                        </div>
                        <div class="flex items-center gap-2 shrink-0">
                          <span class="rounded bg-gray-800/80 px-1.5 py-0.5 text-[10px] font-mono text-gray-400">
                            {strat.sample_games.toLocaleString()} games
                          </span>
                          <span class="rounded px-1.5 py-0.5 text-[10px] font-mono font-bold {strat.build_wpa >= 0 ? 'bg-emerald-950 text-emerald-400 border border-emerald-800/60' : 'bg-rose-950 text-rose-400 border border-rose-800/60'}">
                            {formatWpa(strat.build_wpa)} WPA
                          </span>
                        </div>
                      </button>
                    {/each}
                  </div>
                {/if}
              </div>
            </div>

            <!-- Export to Inventory Column -->
            <div class="flex flex-col items-end">
              <span class="text-[10px] font-semibold uppercase tracking-wider text-transparent select-none mb-1">
                Action
              </span>
              <button
                type="button"
                disabled={exporting}
                onclick={handleExportBuild}
                class="flex items-center gap-1.5 rounded-xl bg-purple-600 px-4 py-2 text-xs font-bold text-white transition-all hover:bg-purple-500 active:scale-95 disabled:opacity-50 cursor-pointer shadow-lg shadow-purple-900/30"
              >
                <span class="icon-[heroicons--arrow-down-tray] h-4 w-4"></span>
                <span>{exporting ? "Exporting..." : "Export to Inventory"}</span>
              </button>
            </div>
          </div>
        </div>

        <!-- Export Toast Notification -->
        {#if exportToast}
          <div class="rounded-xl border p-3 text-xs flex items-center justify-between {exportToast.success ? 'border-emerald-500/50 bg-emerald-950/40 text-emerald-300' : 'border-rose-500/50 bg-rose-950/40 text-rose-300'}">
            <span class="font-medium">{exportToast.message}</span>
            <button type="button" onclick={() => (exportToast = null)} class="text-gray-400 hover:text-white">✕</button>
          </div>
        {/if}

        {#if activeStrategy}
          <!-- Performance Metrics Stats (Container only, no visual card cues) -->
          <div class="flex flex-wrap items-center justify-around gap-4 text-center py-1">
            <div>
              <span class="block text-[11px] font-medium text-gray-400">Build Win Rate</span>
              <strong class="text-lg font-bold text-purple-400">{formatPct(activeStrategy.build_win_rate)}</strong>
            </div>
            <div class="h-8 w-px bg-gray-800"></div>
            <div>
              <span class="block text-[11px] font-medium text-gray-400">WPA</span>
              <strong class="text-lg font-bold {activeStrategy.build_wpa >= 0 ? 'text-emerald-400' : 'text-rose-400'}">
                {formatWpa(activeStrategy.build_wpa)}
              </strong>
            </div>
            <div class="h-8 w-px bg-gray-800"></div>
            <div>
              <span class="block text-[11px] font-medium text-gray-400">{activeStrategy.metric_name}</span>
              <strong class="text-lg font-bold text-cyan-300">{activeStrategy.avg_metric_value}</strong>
            </div>
            <div class="h-8 w-px bg-gray-800"></div>
            <div>
              <span class="block text-[11px] font-medium text-gray-400">Synergy Sample Size</span>
              <strong class="text-lg font-bold text-gray-200">{activeStrategy.sample_games.toLocaleString()} games</strong>
            </div>
          </div>

          <!-- 1. Recommended Items (100% Horizontal Space) -->
          <div class="rounded-xl border border-gray-800 bg-gray-950/60 p-4 space-y-4">
            <div class="flex items-center justify-between border-b border-gray-800 pb-2">
              <span class="text-xs font-semibold uppercase tracking-wider text-gray-300 flex items-center gap-1.5">
                <span class="icon-[heroicons--cube] h-4 w-4 text-amber-400"></span>
                Recommended Items ({activeStrategy.items.length})
              </span>
              <span class="text-[10px] text-gray-500">{activeStrategy.starter_item ? "Starter + 6 Core Items" : "6 Core Items"}</span>
            </div>

            <!-- Horizontal Items Flow: Starter (if present) + 6 Core Recommended Items -->
            <div class="flex flex-col sm:flex-row items-stretch gap-2.5">
              {#if activeStrategy.starter_item}
                <div
                  id="build-starter-item"
                  class="sm:w-36 shrink-0 flex flex-col items-center justify-center p-2.5 rounded-lg border border-amber-500/40 bg-amber-950/20 hover:border-amber-500/60 transition-colors text-center relative group"
                >
                  <div class="relative h-11 w-11 rounded-lg overflow-hidden border border-amber-500/50 bg-gray-950 mb-1">
                    <img src={activeStrategy.starter_item.icon_url} alt={activeStrategy.starter_item.name} class="h-full w-full object-cover" />
                    <span class="absolute left-0 top-0 rounded-br bg-amber-950/90 border-r border-b border-amber-500/40 px-1 text-[8px] font-bold text-amber-300 uppercase">
                      Start
                    </span>
                  </div>
                  <span class="text-[10px] font-semibold text-amber-200 truncate max-w-full">{activeStrategy.starter_item.name}</span>
                  <div class="flex items-center gap-1.5 mt-0.5">
                    <span class="text-[9px] font-mono text-amber-400/90">{activeStrategy.starter_item.price_total}g</span>
                    {#if activeStrategy.starter_item.pick_rate !== undefined && activeStrategy.starter_item.pick_rate > 0}
                      <span class="text-[9px] font-mono text-gray-400">{formatPct(activeStrategy.starter_item.pick_rate)}</span>
                    {/if}
                    {#if activeStrategy.starter_item.wpa !== undefined}
                      <span class="text-[9px] font-mono font-bold {activeStrategy.starter_item.wpa >= 0 ? 'text-emerald-400' : 'text-rose-400'}">
                        {formatWpa(activeStrategy.starter_item.wpa)}
                      </span>
                    {/if}
                  </div>

                  <GameEntityPopover
                    targetId="build-starter-item"
                    name={activeStrategy.starter_item.name}
                    iconUrl={activeStrategy.starter_item.icon_url}
                    description={activeStrategy.starter_item.description}
                    priceTotal={activeStrategy.starter_item.price_total}
                    itemClass={activeStrategy.starter_item.item_class}
                    placement="top"
                  />
                </div>
                <div class="hidden sm:block w-px bg-gray-800 self-stretch my-0.5"></div>
              {/if}

              <!-- 6 Core Recommended Items Grid -->
              <div class="flex-1 grid grid-cols-2 sm:grid-cols-3 lg:grid-cols-6 gap-2">
                {#each activeStrategy.items as item, idx (`item-${item.id}-${idx}`)}
                  <div id={`build-item-${item.id}-${idx}`} class="group relative flex flex-col items-center justify-center p-2 rounded-lg border border-gray-800 bg-gray-900/70 hover:border-gray-700 transition-colors">
                    <div class="relative h-11 w-11 rounded-lg overflow-hidden border border-gray-700 bg-gray-950 mb-1">
                      <img src={item.icon_url} alt={item.name} class="h-full w-full object-cover" />
                      <span class="absolute left-0 top-0 rounded-br bg-gray-950/80 px-1 text-[9px] font-bold text-gray-300" title="Approximate build order">{idx + 1}</span>
                    </div>
                    <span class="text-[10px] font-medium text-gray-200 truncate max-w-full text-center">{item.name}</span>
                    <div class="flex items-center gap-1.5 mt-0.5">
                      <span class="text-[9px] font-mono text-amber-400/90">{item.price_total}g</span>
                      {#if item.pick_rate !== undefined && item.pick_rate > 0}
                        <span class="text-[9px] font-mono text-gray-400">{formatPct(item.pick_rate)}</span>
                      {/if}
                      {#if item.wpa !== undefined}
                        <span class="text-[9px] font-mono font-bold {item.wpa >= 0 ? 'text-emerald-400' : 'text-rose-400'}">
                          {formatWpa(item.wpa)}
                        </span>
                      {/if}
                    </div>

                    <!-- Rich Item Popover -->
                    <GameEntityPopover
                      targetId={`build-item-${item.id}-${idx}`}
                      name={item.name}
                      iconUrl={item.icon_url}
                      description={item.description}
                      priceTotal={item.price_total}
                      itemClass={item.item_class}
                      placement="top"
                    />
                  </div>
                {/each}
              </div>
            </div>

            <!-- Stage-Based Build Pathing Tree -->
            {#if activeStrategy.stages && activeStrategy.stages.length > 0}
              <div class="pt-3 border-t border-gray-800/80">
                <StagePathingTree stages={activeStrategy.stages} />
              </div>
            {/if}

            <!-- Lower section: Alternative Items & Situational Items side-by-side -->
            {#if (activeStrategy.alternative_items && activeStrategy.alternative_items.length > 0) || sortedSituationalItems.length > 0}
              <div class="grid grid-cols-1 {activeStrategy.alternative_items && activeStrategy.alternative_items.length > 0 && sortedSituationalItems.length > 0 ? 'lg:grid-cols-2' : ''} gap-4 pt-3 border-t border-gray-800/80">
                <!-- Alternative Items -->
                {#if activeStrategy.alternative_items && activeStrategy.alternative_items.length > 0}
                  <div>
                    <div class="flex items-center justify-between mb-1.5">
                      <span class="text-[10px] font-semibold uppercase tracking-wider text-gray-400">
                        Alternative Items ({activeStrategy.alternative_items.length})
                      </span>
                      <span class="text-[9px] text-gray-500">Scroll to explore</span>
                    </div>
                    <div class="max-h-44 overflow-y-auto scrollbar-thin space-y-1 pr-1">
                      {#each activeStrategy.alternative_items as altItem, idx (`alt-${altItem.id}-${idx}`)}
                        <div
                          id={`alt-item-${altItem.id}-${idx}`}
                          class="flex items-center justify-between p-1.5 rounded-lg border border-gray-800/80 bg-gray-900/50 hover:bg-gray-800/60 hover:border-gray-700 transition-colors"
                        >
                          <div class="flex items-center gap-2 min-w-0">
                            <img src={altItem.icon_url} alt={altItem.name} class="h-7 w-7 rounded-md object-cover bg-gray-950 border border-gray-700 shrink-0" />
                            <div class="flex flex-col min-w-0">
                              <span class="text-xs font-semibold text-gray-200 truncate">{altItem.name}</span>
                              <span class="text-[9px] font-mono text-amber-400/80">{altItem.price_total}g</span>
                            </div>
                          </div>
                          <div class="flex items-center gap-2 shrink-0">
                            {#if altItem.pick_rate !== undefined && altItem.pick_rate > 0}
                              <span class="rounded bg-gray-800/80 px-1.5 py-0.5 text-[10px] font-mono text-gray-400">
                                {formatPct(altItem.pick_rate)}
                              </span>
                            {/if}
                            {#if altItem.games}<span class="text-[10px] text-gray-500">{altItem.games} games</span>{/if}
                            {#if altItem.wpa !== undefined}
                              <span class="rounded px-1.5 py-0.5 text-[10px] font-mono font-bold {altItem.wpa >= 0 ? 'bg-emerald-950 text-emerald-400 border border-emerald-800/60' : 'bg-rose-950 text-rose-400 border border-rose-800/60'}">
                                {formatWpa(altItem.wpa)}
                              </span>
                            {/if}
                          </div>

                          <GameEntityPopover
                            targetId={`alt-item-${altItem.id}-${idx}`}
                            name={altItem.name}
                            iconUrl={altItem.icon_url}
                            description={altItem.description}
                            priceTotal={altItem.price_total}
                            itemClass={altItem.item_class}
                            placement="top"
                          />
                        </div>
                      {/each}
                    </div>
                  </div>
                {/if}

                <!-- Situational Items -->
                {#if sortedSituationalItems.length > 0}
                  <div>
                    <div class="flex items-center justify-between mb-1.5">
                      <span class="text-[10px] font-semibold uppercase tracking-wider text-gray-400">
                        Situational Items ({sortedSituationalItems.length})
                      </span>
                      <span class="text-[9px] text-gray-500">Sorted by games</span>
                    </div>
                    <div class="flex flex-wrap gap-2 p-2 rounded-lg border border-gray-800/80 bg-gray-900/40">
                      {#each sortedSituationalItems as sit, idx (`sit-${sit.id}-${idx}`)}
                        <div id={`sit-item-${sit.id}-${idx}`} class="relative group">
                          <img
                            src={sit.icon_url}
                            alt={sit.name}
                            class="h-9 w-9 rounded-lg border border-gray-700 bg-gray-950 object-cover hover:border-gray-500 transition-colors cursor-pointer"
                          />
                          {#if sit.situational_tag}
                            <span class="absolute -top-1.5 -right-1 px-1 rounded bg-rose-600/95 text-white text-[8px] font-bold tracking-tight shadow pointer-events-none whitespace-nowrap">
                              {sit.situational_tag}
                            </span>
                          {/if}
                          <GameEntityPopover
                            targetId={`sit-item-${sit.id}-${idx}`}
                            name={sit.name}
                            iconUrl={sit.icon_url}
                            description={sit.description}
                            priceTotal={sit.price_total}
                            itemClass={sit.item_class}
                            games={sit.games}
                            placement="top"
                          />
                        </div>
                      {/each}
                    </div>
                  </div>
                {/if}
              </div>
            {/if}
          </div>

          <!-- 2 & 3. Bottom Row: Summoner Spells & Common Augments (Shared Horizontal Space, Same Y Height) -->
          <div class="grid grid-cols-1 md:grid-cols-2 gap-6 items-stretch">
            <!-- 2. Summoner Spells Card -->
            <div class="rounded-xl border border-gray-800 bg-gray-950/60 p-4 flex flex-col h-full space-y-3">
              <div class="flex items-center justify-between border-b border-gray-800 pb-2">
                <span class="text-xs font-semibold uppercase tracking-wider text-gray-300 flex items-center gap-1.5">
                  <span class="icon-[heroicons--bolt] h-4 w-4 text-cyan-400"></span>
                  Summoner Spells
                </span>
                <span class="text-[10px] text-gray-500">Optimal Pair</span>
              </div>
              <div class="grid grid-cols-2 gap-3 pt-1">
                {#each activeStrategy.spells as spell, idx (`spell-${spell.id}-${idx}`)}
                  <div class="flex items-center gap-2.5 p-2.5 rounded-lg border border-gray-800 bg-gray-900/70">
                    <img src={spell.icon_url} alt={spell.name} class="h-9 w-9 rounded-lg border border-gray-700 bg-gray-950 object-cover shrink-0" />
                    <div class="flex flex-col min-w-0">
                      <span class="text-xs font-semibold text-gray-200 truncate">{spell.name}</span>
                      <div class="flex items-center gap-1.5 mt-0.5">
                        {#if spell.win_rate !== undefined}
                          <span class="text-[10px] text-gray-400">{formatPct(spell.win_rate)} WR</span>
                        {/if}
                        {#if spell.wpa !== undefined}
                          <span class="text-[10px] font-mono font-bold {spell.wpa >= 0 ? 'text-emerald-400' : 'text-rose-400'}">
                            {formatWpa(spell.wpa)}
                          </span>
                        {/if}
                      </div>
                    </div>
                  </div>
                {/each}
              </div>

              <!-- Other Summoner Spells with WPA -->
              {#if otherSpells.length > 0}
                <div class="pt-2 border-t border-gray-800/80 flex-1 flex flex-col">
                  <div class="flex items-center justify-between mb-1.5">
                    <span class="text-[10px] font-semibold uppercase tracking-wider text-gray-400">
                      All Other Summoner Spells
                    </span>
                    <span class="text-[9px] text-gray-500">Win Rate & WPA</span>
                  </div>
                  <div class="grid grid-cols-2 gap-2 overflow-y-auto max-h-48 scrollbar-thin">
                    {#each otherSpells as spell, idx (`other-spell-${spell.id}-${idx}`)}
                      <div class="flex items-center justify-between p-1.5 rounded-lg border border-gray-800/80 bg-gray-900/40">
                        <div class="flex items-center gap-2 min-w-0">
                          <img src={spell.icon_url} alt={spell.name} class="h-6 w-6 rounded bg-gray-950 object-cover shrink-0" />
                          <span class="text-[11px] font-medium text-gray-300 truncate">{spell.name}</span>
                        </div>
                        {#if spell.wpa !== undefined}
                          <span class="text-[10px] font-mono font-semibold {spell.wpa >= 0 ? 'text-emerald-400' : 'text-rose-400'} shrink-0">
                            {formatWpa(spell.wpa)}
                          </span>
                        {/if}
                      </div>
                    {/each}
                  </div>
                </div>
              {/if}
            </div>

            <!-- 3. Common Augments Associated With Build Card -->
            <div class="rounded-xl border border-gray-800 bg-gray-950/60 p-4 flex flex-col h-full space-y-3">
              <div class="flex items-center justify-between border-b border-gray-800 pb-2">
                <span class="text-xs font-semibold uppercase tracking-wider text-gray-300 flex items-center gap-1.5">
                  <span class="icon-[flowbite--badge-check-outline] h-4 w-4 text-purple-400"></span>
                  Common Augments ({activeStrategy.augments?.length ?? 0})
                </span>
                <span class="text-[10px] text-gray-500">Associated With Core</span>
              </div>

              {#if activeStrategy.augments && activeStrategy.augments.length > 0}
                <div class="grid grid-cols-3 gap-2 overflow-y-auto max-h-64 scrollbar-thin flex-1">
                  {#each activeStrategy.augments as aug, idx (`build-aug-${aug.id}-${idx}`)}
                    <button
                      type="button"
                      id={`build-aug-${aug.id}-${idx}`}
                      onclick={() => handleSelectBuildAugment(aug)}
                      class="group relative flex flex-col items-center p-2 rounded-lg border border-gray-800 bg-gray-900/70 hover:border-gray-700 transition-colors cursor-pointer hover:border-purple-500/60 hover:bg-purple-950/20 w-full text-center"
                      title={`Click to simulate or track ${aug.name}`}
                    >
                      <div class="relative mb-1">
                        <AugmentIcon
                          src={aug.icon_url}
                          name={aug.name}
                          rarity={aug.rarity}
                          class="h-11 w-11"
                          roundedClass="rounded-lg"
                        />
                      </div>
                      <span class="text-[10px] font-medium text-gray-200 truncate max-w-full text-center" title={aug.name}>
                        {aug.name}
                      </span>
                      <div class="flex items-center gap-1.5 mt-0.5">
                        {#if aug.pick_rate !== undefined && aug.pick_rate > 0}
                          <span class="text-[9px] font-mono text-purple-300" title={`${aug.games ?? 0} games (${formatPct(aug.pick_rate)} pick rate)`}>
                            {formatPct(aug.pick_rate)}
                          </span>
                        {:else if aug.games}
                          <span class="text-[9px] font-mono text-gray-400" title={`${aug.games} games`}>
                            {aug.games}g
                          </span>
                        {/if}
                        {#if aug.wpa !== undefined}
                          <span
                            class="text-[9px] font-mono font-bold {aug.wpa >= 0 ? 'text-emerald-400' : 'text-rose-400'}"
                            title={`Win rate: ${formatPct(aug.win_rate ?? 0)} (${formatWpa(aug.wpa)} WPA)`}
                          >
                            {formatWpa(aug.wpa)}
                          </span>
                        {/if}
                      </div>

                      <!-- Rich Augment Popover -->
                      <GameEntityPopover
                        targetId={`build-aug-${aug.id}-${idx}`}
                        name={aug.name}
                        iconUrl={aug.icon_url}
                        description={aug.tooltip}
                        rarity={aug.rarity}
                        placement="top"
                      />
                    </button>
                  {/each}
                </div>
              {:else}
                <div class="flex flex-1 items-center justify-center rounded-lg border border-dashed border-gray-800 text-xs text-gray-500 min-h-24">
                  No augment data sampled for this build yet.
                </div>
              {/if}
            </div>
          </div>
        {/if}
      </div>
    {:else if loadingBuild}
      <div class="flex h-64 items-center justify-center rounded-2xl border border-gray-800 bg-gray-900/40 text-sm text-gray-400">
        <span class="icon-[heroicons--arrow-path] h-5 w-5 animate-spin mr-2 text-purple-400"></span>
        Mining champion build statistics...
      </div>
    {/if}
      </div>

      <!-- Right Column: Dedicated Complete Augments Card -->
      <div class="xl:col-span-5 space-y-4 min-w-0">
        <MayhemAugmentsCard
          championId={selectedChampionId}
          championName={inspectedBuild?.champion_name || currentSelectedChampion?.name || ""}
          championWinRate={inspectedBuild?.champion_win_rate ?? 0.5}
          totalGames={inspectedBuild?.total_games ?? 0}
          hideLowPickrate={hideLowPickrate}
        />
      </div>
    </div>

  </div>

  <!-- Floating Quick Tooltip on Roster Hover -->
  {#if hoverTooltip.visible && hoverTooltip.summary}
    {@const topStrat = hoverTooltip.summary.strategies[0]}
    <div
      class="pointer-events-none fixed z-50 w-72 rounded-xl border border-purple-500/50 bg-gray-900/95 p-3.5 shadow-2xl backdrop-blur-md text-xs -translate-x-1/2"
      style={`left: ${hoverTooltip.x}px; top: ${hoverTooltip.y}px;`}
    >
      <div class="flex items-center gap-2.5 pb-2 border-b border-gray-800">
        <img src={hoverTooltip.summary.icon_url} alt="" class="h-8 w-8 rounded-lg object-cover border border-gray-700 bg-gray-950" />
        <div class="flex flex-col">
          <span class="font-bold text-gray-100">{hoverTooltip.summary.champion_name}</span>
          <div class="flex items-center gap-1.5 text-[10px]">
            <span class="text-gray-300">{formatPct(hoverTooltip.summary.champion_win_rate)} WR</span>
            <span class="font-bold {hoverTooltip.summary.champion_wpa >= 0 ? 'text-emerald-400' : 'text-rose-400'}">
              {formatWpa(hoverTooltip.summary.champion_wpa)} WPA
            </span>
          </div>
        </div>
      </div>

      {#if topStrat}
        <div class="mt-2.5 space-y-2">
          <!-- Top Items row -->
          <div>
            <div class="flex items-center justify-between mb-1">
              <span class="text-[10px] font-semibold text-gray-400">Optimal Items:</span>
              {#if topStrat.starter_item}
                <span class="text-[9px] text-amber-400/90 truncate max-w-[140px]">Starter: {topStrat.starter_item.name}</span>
              {/if}
            </div>
            <div class="flex items-center gap-1">
              {#if topStrat.starter_item}
                <img src={topStrat.starter_item.icon_url} alt={topStrat.starter_item.name} title={`Starter: ${topStrat.starter_item.name}`} class="h-6 w-6 rounded border border-amber-500/50 bg-gray-950 object-cover" />
              {/if}
              {#each topStrat.items.slice(0, 5) as item}
                <img src={item.icon_url} alt={item.name} title={item.name} class="h-6 w-6 rounded border border-gray-700 bg-gray-950 object-cover" />
              {/each}
            </div>
          </div>
          <!-- Spells & Augments row -->
          <div class="flex items-center justify-between pt-1 border-t border-gray-800/60">
            <div class="flex items-center gap-1">
              {#each topStrat.spells as spell}
                <img src={spell.icon_url} alt={spell.name} title={spell.name} class="h-5 w-5 rounded bg-gray-950 object-cover" />
              {/each}
            </div>
            <div class="flex items-center gap-1">
              {#each topStrat.augments.slice(0, 4) as aug}
                <AugmentIcon src={aug.icon_url} name={aug.name} rarity={aug.rarity} class="h-5 w-5" />
              {/each}
            </div>
          </div>
        </div>
      {/if}
    </div>
  {/if}

  <!-- Mayhem Encyclopedia Modal -->
  <MayhemEncyclopediaModal
    isOpen={isEncyclopediaOpen}
    onClose={() => isEncyclopediaOpen = false}
    onSimulateAugment={(aug) => toggleSimulatedAugment(aug)}
  />
</div>
