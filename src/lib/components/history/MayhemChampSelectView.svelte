<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import {
    getChampSelectSession,
    getMayhemChampionBuild,
    getMayhemTeamBuilds,
    exportMayhemBuild,
  } from "../../api/champ_select";
  import { getChampions } from "../../api/champions";
  import type { ChampionSummary } from "../../types/champions";
  import type {
    ChampSelectSession,
    MayhemChampionBuildSummary,
    MayhemBuildStrategy,
  } from "../../types/champ_select";
  import ChampionCarousel from "../sidebar/ChampionCarousel.svelte";
  import AugmentIcon from "../common/AugmentIcon.svelte";
  import GameEntityPopover from "../common/GameEntityPopover.svelte";
  import MayhemAugmentsCard from "./MayhemAugmentsCard.svelte";
  import { getSummonerProfile } from "../../api/profile";
  import { logger } from "../../utils/logger";

  let session = $state<ChampSelectSession | null>(null);
  let isLcuConnected = $state(false);
  let teamBuilds = $state<Record<number, MayhemChampionBuildSummary>>({});
  let allChampions = $state<ChampionSummary[]>([]);
  let selectedChampionId = $state<number>(22); // Default to Ashe or picked champ
  let selectedStrategyId = $state<string>("win_rate");
  let manualChampionBuild = $state<MayhemChampionBuildSummary | null>(null);
  let loadingBuild = $state(false);
  let strategyDropdownOpen = $state(false);
  let searchQuery = $state("");
  let exporting = $state(false);
  let exportToast = $state<{ message: string; success: boolean } | null>(null);
  let hideLowPickrate = $state(
    typeof window !== "undefined" &&
      localStorage.getItem("lcu-statistics-hide-low-pickrate") === "true"
  );
  let hoverTooltip = $state<{
    visible: boolean;
    x: number;
    y: number;
    summary: MayhemChampionBuildSummary | null;
  }>({ visible: false, x: 0, y: 0, summary: null });

  function toggleHideLowPickrate() {
    hideLowPickrate = !hideLowPickrate;
    if (typeof window !== "undefined") {
      localStorage.setItem("lcu-statistics-hide-low-pickrate", String(hideLowPickrate));
      window.dispatchEvent(new Event("storage"));
    }
  }

  function handleStorageChange() {
    if (typeof window !== "undefined") {
      hideLowPickrate =
        localStorage.getItem("lcu-statistics-hide-low-pickrate") === "true";
    }
  }

  let unlistenSession: UnlistenFn | null = null;
  let unlistenPhase: UnlistenFn | null = null;
  let unlistenConnected: UnlistenFn | null = null;
  let unlistenDisconnected: UnlistenFn | null = null;

  const isInChampSelect = $derived(
    Boolean(
      session &&
        session.myTeam &&
        session.myTeam.length > 0 &&
        (session.queueId === 2400 || session.benchEnabled)
    )
  );

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

  const mayhemAllChampions = $derived(
    allChampions.filter((c) => c.id > 0 && !c.id.toString().startsWith("600"))
  );

  const carouselFavoriteChampions = $derived(
    mayhemAllChampions
      .filter((c) => c.is_favorite)
      .sort((a, b) => a.name.localeCompare(b.name))
  );

  const carouselNonFavoriteChampions = $derived(
    mayhemAllChampions
      .filter((c) => !c.is_favorite)
      .sort((a, b) => a.name.localeCompare(b.name))
  );

  const carouselSearchedChampions = $derived(
    searchQuery.trim()
      ? mayhemAllChampions
          .filter(
            (c) =>
              c.name.toLowerCase().includes(searchQuery.trim().toLowerCase()) ||
              c.alias?.toLowerCase().includes(searchQuery.trim().toLowerCase())
          )
          .sort((a, b) => {
            const aFav = a.is_favorite ? 1 : 0;
            const bFav = b.is_favorite ? 1 : 0;
            if (aFav !== bFav) return bFav - aFav;
            return a.name.localeCompare(b.name);
          })
      : []
  );

  const currentSelectedChampion = $derived(
    allChampions.find((c) => c.id === selectedChampionId) ?? null
  );

  async function checkLcuConnection() {
    try {
      const prof = await getSummonerProfile();
      isLcuConnected = Boolean(prof && prof.display_name);
    } catch {
      isLcuConnected = false;
    }
  }

  onMount(async () => {
    try {
      allChampions = await getChampions();
    } catch (err) {
      logger.error("frontend.mayhem_champ_select", "Failed loading champions", err);
    }

    // Check initial active session and connection
    await refreshSession();

    // Listen to real-time events
    try {
      unlistenSession = await listen("lcu:champ-select-session", async (event) => {
        const data = event.payload as ChampSelectSession | null;
        if (data && data.myTeam && data.myTeam.length > 0) {
          session = data;
          isLcuConnected = true;
          await handleSessionUpdated(data);
        } else {
          session = null;
        }
      });

      unlistenPhase = await listen("lcu:gameflow-phase", async (event) => {
        const phase = event.payload as string;
        if (phase !== "ChampSelect") {
          session = null;
        } else {
          await refreshSession();
        }
      });

      unlistenConnected = await listen("lcu:connected", async () => {
        isLcuConnected = true;
        await refreshSession();
      });

      unlistenDisconnected = await listen("lcu:disconnected", () => {
        isLcuConnected = false;
        session = null;
      });
    } catch (err) {
      logger.error("frontend.mayhem_champ_select", "Failed attaching event listeners", err);
    }

    // If not in champ select, load default champion build and augments
    if (!isInChampSelect && selectedChampionId > 0) {
      await loadSingleChampionBuild(selectedChampionId);
    }

    if (typeof window !== "undefined") {
      window.addEventListener("storage", handleStorageChange);
    }
  });

  onDestroy(() => {
    if (typeof window !== "undefined") {
      window.removeEventListener("storage", handleStorageChange);
    }
    if (unlistenSession) unlistenSession();
    if (unlistenPhase) unlistenPhase();
    if (unlistenConnected) unlistenConnected();
    if (unlistenDisconnected) unlistenDisconnected();
  });

  async function refreshSession() {
    try {
      const activeSession = await getChampSelectSession();
      if (activeSession && activeSession.myTeam && activeSession.myTeam.length > 0) {
        session = activeSession;
        isLcuConnected = true;
        await handleSessionUpdated(activeSession);
      } else {
        session = null;
        await checkLcuConnection();
      }
    } catch (err) {
      isLcuConnected = false;
      session = null;
      logger.warn("frontend.mayhem_champ_select", "Champ select session unavailable", err);
    }
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
        const builds = await getMayhemTeamBuilds(ids);
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
      }
    } else if (ids.length > 0 && (!selectedChampionId || selectedChampionId === 0)) {
      selectedChampionId = ids[0];
    }
  }

  async function loadSingleChampionBuild(championId: number) {
    if (!championId || championId <= 0) return;
    loadingBuild = true;
    try {
      const build = await getMayhemChampionBuild(championId);
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

  function handleSelectChampion(champId: number) {
    selectedChampionId = champId;
    searchQuery = "";
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

      <!-- Status Indicators & Low Pick-Rate Control -->
      <div class="flex items-center gap-2">
        <!-- 0. Hide Low Pick-Rate Filter Toggle -->
        <button
          type="button"
          onclick={toggleHideLowPickrate}
          title={hideLowPickrate
            ? "Hide Low Pick-rate: ON (Filtering out off-meta champions <1% and items <0.2%; augments always preserved)"
            : "Hide Low Pick-rate: OFF (Showing all data)"}
          class="inline-flex items-center gap-1.5 rounded-full border px-2.5 py-0.5 text-[11px] font-medium transition-all cursor-pointer {hideLowPickrate
            ? 'border-purple-500/50 bg-purple-950/70 text-purple-300 hover:bg-purple-900/60 shadow-sm'
            : 'border-gray-800 bg-gray-900/60 text-gray-400 hover:border-gray-700 hover:text-gray-300'}"
        >
          <span class="icon-[heroicons--funnel] h-3 w-3 {hideLowPickrate ? 'text-purple-400' : 'text-gray-500'}"></span>
          <span>Hide Low Pick-Rate</span>
          <span
            class="ml-0.5 rounded px-1 text-[9px] font-mono font-semibold {hideLowPickrate
              ? 'bg-purple-800/80 text-purple-100'
              : 'bg-gray-800 text-gray-500'}"
          >
            {hideLowPickrate ? "ON" : "OFF"}
          </span>
        </button>

        <!-- 1. LCU Client Status Indicator -->
        {#if isLcuConnected}
          <span class="inline-flex items-center gap-1.5 rounded-full border border-emerald-500/30 bg-emerald-950/60 px-2.5 py-0.5 text-[11px] font-medium text-emerald-300">
            <span class="h-1.5 w-1.5 rounded-full bg-emerald-400"></span>
            LCU Client: Connected
          </span>
        {:else}
          <span class="inline-flex items-center gap-1.5 rounded-full border border-rose-500/30 bg-rose-950/60 px-2.5 py-0.5 text-[11px] font-medium text-rose-300">
            <span class="h-1.5 w-1.5 rounded-full bg-rose-400"></span>
            LCU Client: Offline
          </span>
        {/if}

        <!-- 2. Champ Select Status Indicator -->
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
                onclick={() => member.championId > 0 && handleSelectChampion(member.championId)}
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
                  onclick={() => handleSelectChampion(bench.championId)}
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

    <!-- Champion Carousel Horizontal Bar Scrolling Card on top of Main Champion Card -->
    <div class="rounded-2xl border border-gray-800 bg-gray-900/60 p-3.5 shadow-xl">
      <div class="flex items-center justify-between mb-2.5 px-1">
        <div class="flex items-center gap-2">
          <span class="icon-[heroicons--sparkles] h-4 w-4 text-purple-400"></span>
          <h2 class="text-xs font-semibold uppercase tracking-wider text-gray-400">
            Champions ({mayhemAllChampions.length})
          </h2>
          {#if currentSelectedChampion}
            <span class="rounded bg-purple-950/80 border border-purple-800/80 px-2 py-0.5 text-[11px] font-medium text-purple-300">
              Selected: <strong class="text-white">{currentSelectedChampion.name}</strong>
            </span>
          {/if}
        </div>
        <div class="relative w-52">
          <span class="icon-[heroicons--magnifying-glass] pointer-events-none absolute left-2.5 top-1/2 -translate-y-1/2 h-3.5 w-3.5 text-gray-500"></span>
          <input
            type="text"
            placeholder="Quick champion filter..."
            bind:value={searchQuery}
            class="w-full rounded-lg border border-gray-700 bg-gray-950/80 pl-8 pr-2.5 py-1 text-xs text-gray-200 placeholder-gray-500 focus:border-purple-500 focus:outline-none"
          />
          {#if searchQuery}
            <button
              type="button"
              title="Clear filter"
              onclick={() => (searchQuery = "")}
              class="absolute right-2 top-1/2 -translate-y-1/2 text-gray-400 hover:text-gray-200 cursor-pointer"
            >
              <span class="icon-[heroicons--x-mark] h-3.5 w-3.5"></span>
            </button>
          {/if}
        </div>
      </div>

      <div class="rounded-xl border border-gray-800/80 bg-gray-950/40 p-2">
        <ChampionCarousel
          searchedChampions={carouselSearchedChampions}
          favoriteChampions={carouselFavoriteChampions}
          nonFavoriteChampions={carouselNonFavoriteChampions}
          selectedChampion={currentSelectedChampion}
          {searchQuery}
          isRuneMismatch={false}
          activeRuneChampionId={null}
          showAllChampions={false}
          onSelectChampion={(champ) => champ && handleSelectChampion(champ.id)}
        />
      </div>
    </div>

    <!-- Main Champion Card alongside Dedicated Complete Augments Card -->
    <div class="grid grid-cols-1 xl:grid-cols-12 gap-6 items-start">
      <!-- Left Column: Main Champion Card -->
      <div class="xl:col-span-7 space-y-6 min-w-0">

    <!-- Section 2: Detailed Inspected Champion & Strategy Analysis -->
    {#if inspectedBuild}
      <div class="rounded-2xl border border-gray-800 bg-gray-900/60 p-6 shadow-xl space-y-6">
        <!-- Champion Hero Banner & Strategy Selector Bar -->
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

          <!-- Strategy Selector & Export Action -->
          <div class="flex flex-wrap items-center gap-3">
            <!-- Strategy Dropdown Selector -->
            <div class="relative">
              <button
                type="button"
                onclick={() => (strategyDropdownOpen = !strategyDropdownOpen)}
                class="flex items-center gap-2.5 rounded-xl border border-gray-700 bg-gray-900 px-3.5 py-2 text-xs font-medium text-gray-200 transition-colors hover:border-purple-500 focus:outline-none cursor-pointer"
              >
                <span class="text-gray-400">Strategy:</span>
                <span class="font-bold text-white">{activeStrategy?.label ?? "Select Strategy"}</span>
                {#if activeStrategy}
                  <span class="rounded bg-gray-800 px-1.5 py-0.5 text-[10px] font-mono text-purple-300">
                    {activeStrategy.avg_metric_value}
                  </span>
                  <span class="rounded px-1.5 py-0.5 text-[10px] font-bold font-mono {activeStrategy.build_wpa >= 0 ? 'bg-emerald-950 text-emerald-400 border border-emerald-800/60' : 'bg-rose-950 text-rose-400 border border-rose-800/60'}">
                    {formatWpa(activeStrategy.build_wpa)} WPA
                  </span>
                  <span class="rounded bg-gray-800/80 px-1.5 py-0.5 text-[10px] font-mono text-gray-400">
                    {activeStrategy.sample_games.toLocaleString()} games
                  </span>
                {/if}
                <span class="icon-[heroicons--chevron-down] h-4 w-4 text-gray-400"></span>
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
                      <div class="flex flex-col">
                        <span class="font-semibold">{strat.label}</span>
                        <span class="text-[10px] text-gray-400">{strat.avg_metric_value}</span>
                      </div>
                      <div class="flex items-center gap-2">
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

            <!-- Export to Inventory Button -->
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

        <!-- Export Toast Notification -->
        {#if exportToast}
          <div class="rounded-xl border p-3 text-xs flex items-center justify-between {exportToast.success ? 'border-emerald-500/50 bg-emerald-950/40 text-emerald-300' : 'border-rose-500/50 bg-rose-950/40 text-rose-300'}">
            <span class="font-medium">{exportToast.message}</span>
            <button type="button" onclick={() => (exportToast = null)} class="text-gray-400 hover:text-white">✕</button>
          </div>
        {/if}

        <!-- Build Cards Grid: Items, Spells, Augments -->
        {#if activeStrategy}
          <div class="grid grid-cols-1 lg:grid-cols-2 gap-6">
            <!-- 1. Recommended Items -->
            <div class="rounded-xl border border-gray-800 bg-gray-950/60 p-4 space-y-3">
              <div class="flex items-center justify-between border-b border-gray-800 pb-2">
                <span class="text-xs font-semibold uppercase tracking-wider text-gray-300 flex items-center gap-1.5">
                  <span class="icon-[heroicons--cube] h-4 w-4 text-amber-400"></span>
                  Recommended Items ({activeStrategy.items.length})
                </span>
                <span class="text-[10px] text-gray-500">{activeStrategy.starter_item ? "Starter + 6 Items" : "6 Items"}</span>
              </div>

              <!-- Starter item shown separately above the 6 items -->
              {#if activeStrategy.starter_item}
                <div
                  id="build-starter-item"
                  class="flex items-center justify-between rounded-lg border border-amber-500/30 bg-amber-950/20 p-2 hover:border-amber-500/50 transition-colors"
                >
                  <div class="flex items-center gap-2.5 min-w-0">
                    <div class="relative h-9 w-9 rounded-lg overflow-hidden border border-amber-500/40 bg-gray-950 shrink-0">
                      <img src={activeStrategy.starter_item.icon_url} alt={activeStrategy.starter_item.name} class="h-full w-full object-cover" />
                    </div>
                    <div class="flex flex-col min-w-0">
                      <div class="flex items-center gap-1.5">
                        <span class="text-xs font-bold text-amber-200 truncate">{activeStrategy.starter_item.name}</span>
                        <span class="rounded bg-amber-900/60 px-1 py-0.2 text-[8px] font-semibold text-amber-300 uppercase tracking-wider">
                          Starter
                        </span>
                      </div>
                      <span class="text-[10px] font-mono text-amber-400/80">{activeStrategy.starter_item.price_total}g</span>
                    </div>
                  </div>
                  <div class="flex items-center gap-2">
                    {#if activeStrategy.starter_item.pick_rate !== undefined && activeStrategy.starter_item.pick_rate > 0}
                      <span class="rounded bg-gray-800/80 px-1.5 py-0.5 text-[10px] font-mono text-gray-400">
                        {formatPct(activeStrategy.starter_item.pick_rate)}
                      </span>
                    {/if}
                    {#if activeStrategy.starter_item.wpa !== undefined}
                      <span class="rounded px-1.5 py-0.5 text-[10px] font-mono font-bold {activeStrategy.starter_item.wpa >= 0 ? 'bg-emerald-950 text-emerald-400 border border-emerald-800/60' : 'bg-rose-950 text-rose-400 border border-rose-800/60'}">
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
              {/if}

              <!-- 6 Core Recommended Items Grid -->
              <div class="grid grid-cols-3 gap-2">
                {#each activeStrategy.items as item, idx (`item-${item.id}-${idx}`)}
                  <div id={`build-item-${item.id}-${idx}`} class="group relative flex flex-col items-center p-2 rounded-lg border border-gray-800 bg-gray-900/70 hover:border-gray-700 transition-colors">
                    <div class="relative h-11 w-11 rounded-lg overflow-hidden border border-gray-700 bg-gray-950 mb-1">
                      <img src={item.icon_url} alt={item.name} class="h-full w-full object-cover" />
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

              <!-- Compact scrollable list of alternative items -->
              {#if activeStrategy.alternative_items && activeStrategy.alternative_items.length > 0}
                <div class="pt-2 border-t border-gray-800/80">
                  <div class="flex items-center justify-between mb-1.5">
                    <span class="text-[10px] font-semibold uppercase tracking-wider text-gray-400">
                      Alternative & Situational Items ({activeStrategy.alternative_items.length})
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
            </div>

            <!-- 2. Summoner Spells -->
            <div class="rounded-xl border border-gray-800 bg-gray-950/60 p-4 space-y-3">
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
                    <img src={spell.icon_url} alt={spell.name} class="h-10 w-10 rounded-lg object-cover bg-gray-950 border border-gray-700 shrink-0" />
                    <div class="flex flex-col min-w-0">
                      <div class="flex items-center justify-between gap-1">
                        <span class="text-xs font-bold text-gray-100 truncate">{spell.name}</span>
                        {#if spell.wpa !== undefined}
                          <span class="rounded px-1 py-0.2 text-[9px] font-mono font-bold {spell.wpa >= 0 ? 'bg-emerald-950 text-emerald-400 border border-emerald-800/60' : 'bg-rose-950 text-rose-400 border border-rose-800/60'}">
                            {formatWpa(spell.wpa)}
                          </span>
                        {/if}
                      </div>
                      <span class="text-[10px] text-gray-400 line-clamp-2">{spell.description || "Active spell"}</span>
                    </div>
                  </div>
                {/each}
              </div>

              <!-- Other Summoner Spells with WPA -->
              {#if otherSpells.length > 0}
                <div class="pt-2 border-t border-gray-800/80 space-y-1.5">
                  <div class="flex items-center justify-between">
                    <span class="text-[10px] font-semibold uppercase tracking-wider text-gray-400">
                      All Other Summoner Spells
                    </span>
                    <span class="text-[9px] text-gray-500">Spell WPA</span>
                  </div>
                  <div class="max-h-48 overflow-y-auto scrollbar-thin space-y-1 pr-1">
                    {#each otherSpells as spell (spell.id)}
                      <div class="flex items-center justify-between p-1.5 rounded-lg border border-gray-800/80 bg-gray-900/60 hover:bg-gray-800/60 transition-colors">
                        <div class="flex items-center gap-2 min-w-0">
                          <img src={spell.icon_url} alt={spell.name} class="h-6 w-6 rounded-md object-cover bg-gray-950 border border-gray-700 shrink-0" />
                          <div class="flex flex-col min-w-0">
                            <span class="text-xs font-semibold text-gray-200 truncate">{spell.name}</span>
                            {#if spell.games}<span class="text-[10px] text-gray-500">{spell.games} games</span>{/if}
                          </div>
                        </div>
                        <div class="flex items-center gap-2 shrink-0">
                          {#if spell.win_rate !== undefined}
                            <span class="text-[10px] text-gray-400">{formatPct(spell.win_rate)} WR</span>
                          {/if}
                          {#if spell.wpa !== undefined}
                            <span class="rounded px-1.5 py-0.5 text-[10px] font-mono font-bold {spell.wpa >= 0 ? 'bg-emerald-950 text-emerald-400 border border-emerald-800/60' : 'bg-rose-950 text-rose-400 border border-rose-800/60'}">
                              {formatWpa(spell.wpa)}
                            </span>
                          {/if}
                        </div>
                      </div>
                    {/each}
                  </div>
                </div>
              {/if}
            </div>
          </div>

          <!-- Performance Metrics Bar -->
          <div class="rounded-xl border border-gray-800 bg-gray-950/40 p-4 flex flex-wrap items-center justify-around gap-4 text-center">
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
</div>
