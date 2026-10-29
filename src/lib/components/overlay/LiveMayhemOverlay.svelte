<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import {
    getLiveBuildAdvice,
    getMayhemChampionBuild,
    scanLiveAugmentCards,
    emitOverlayState,
  } from "../../api/champ_select";
  import { requestStatistics } from "../../utils/statistics-data";
  import type { ServerStatisticRow } from "../../api/agent";
  import type {
    LiveGameState,
    MayhemAugmentItem,
    MayhemChampionBuildSummary,
    MayhemLiveAdvice,
  } from "../../types/champ_select";
  import AugmentIcon from "../common/AugmentIcon.svelte";
  import { logger } from "../../utils/logger";

  let liveGameState = $state<LiveGameState | null>(null);
  let championId = $state<number>(0);
  let championName = $state<string>("");
  let selectedStrategyId = $state<string>("highest_wr");
  let augmentIds = $state<string[]>([]);
  let offeredAugments = $state<MayhemAugmentItem[]>([]);
  let buildSummary = $state<MayhemChampionBuildSummary | null>(null);
  let liveAdvice = $state<MayhemLiveAdvice | null>(null);
  let topCombos = $state<ServerStatisticRow[]>([]);
  let scanning = $state<boolean>(false);
  let hudExpanded = $state<boolean>(true);
  let lastScannedLevel = $state<number>(0);

  let unlistenState: UnlistenFn | null = null;
  let unlistenSync: UnlistenFn | null = null;

  const activeStrategy = $derived(
    buildSummary?.strategies.find((s) => s.id === selectedStrategyId) ??
      buildSummary?.strategies[0] ??
      null
  );

  function getTierBorderClass(tier: string | undefined): string {
    const t = (tier || "").toLowerCase();
    if (t.includes("prismatic")) {
      return "border-purple-400 shadow-[0_0_12px_rgba(217,70,239,0.4)]";
    }
    if (t.includes("gold")) {
      return "border-amber-400 shadow-[0_0_10px_rgba(245,158,11,0.35)]";
    }
    return "border-cyan-400/80 shadow-[0_0_8px_rgba(34,211,238,0.25)]";
  }

  function getTierBadgeColor(tier: string | undefined): string {
    const t = (tier || "").toLowerCase();
    if (t.includes("prismatic")) return "bg-purple-950/80 text-purple-300 border-purple-500/50";
    if (t.includes("gold")) return "bg-amber-950/80 text-amber-300 border-amber-500/50";
    return "bg-cyan-950/80 text-cyan-300 border-cyan-500/50";
  }

  async function loadBuildData(champId: number, augs: string[]) {
    if (champId <= 0) return;
    try {
      buildSummary = await getMayhemChampionBuild(champId, augs, "v2");
      if (liveGameState && activeStrategy) {
        liveAdvice = await getLiveBuildAdvice({
          championId: champId,
          strategyId: selectedStrategyId,
          currentGold: liveGameState.current_gold,
          inventoryItemIds: liveGameState.items.map((i) => i.item_id),
          level: liveGameState.level,
          gameTime: liveGameState.game_time,
          augmentIds: augs,
        });
      }
      loadTopCombos(champId);
    } catch (err) {
      logger.warn("overlay", "Failed loading build data", err);
    }
  }

  async function loadTopCombos(champId: number) {
    try {
      const res = await requestStatistics({
        scope: "global",
        kind: "combo",
        champion_id: champId,
        sort_key: "wpa",
        descending: true,
        limit: 15,
      });
      topCombos = res.rows || [];
    } catch (err) {
      logger.warn("overlay", "Failed fetching top combos", err);
    }
  }

  async function handleScanCards() {
    if (scanning) return;
    scanning = true;
    try {
      const cards = await scanLiveAugmentCards(championId > 0 ? championId : undefined, augmentIds);
      if (cards && cards.length > 0) {
        offeredAugments = cards;
      }
    } catch (err) {
      logger.warn("overlay", "Scan cards failed", err);
    } finally {
      scanning = false;
    }
  }

  function selectAugment(item: MayhemAugmentItem) {
    if (!augmentIds.includes(item.name)) {
      if (augmentIds.length >= 4) {
        augmentIds = [...augmentIds.slice(1), item.name];
      } else {
        augmentIds = [...augmentIds, item.name];
      }
      emitOverlayState({
        championId,
        championName,
        strategyId: selectedStrategyId,
        augmentIds,
      });
      loadBuildData(championId, augmentIds);
    }
    offeredAugments = [];
  }

  function removeAugment(name: string) {
    augmentIds = augmentIds.filter((id) => id !== name);
    emitOverlayState({
      championId,
      championName,
      strategyId: selectedStrategyId,
      augmentIds,
    });
    loadBuildData(championId, augmentIds);
  }

  let rectInterval: ReturnType<typeof setInterval> | null = null;

  function reportInteractiveRects() {
    const dpr = window.devicePixelRatio || 1;
    const elements = document.querySelectorAll('[data-overlay-interactive="true"]');
    const rects: Array<{ x: number; y: number; width: number; height: number }> = [];

    elements.forEach((el) => {
      const r = el.getBoundingClientRect();
      const w = Math.round(r.width * dpr);
      const h = Math.round(r.height * dpr);
      if (w > 0 && h > 0) {
        rects.push({
          x: Math.round(r.left * dpr),
          y: Math.round(r.top * dpr),
          width: w,
          height: h,
        });
      }
    });

    void emitOverlayState({
      interactiveRects: rects,
      hasCards: offeredAugments.length > 0,
      hudExpanded,
    });
  }

  $effect(() => {
    // Track dependencies
    void offeredAugments;
    void hudExpanded;
    void liveAdvice;
    void buildSummary;

    const frameId = requestAnimationFrame(() => {
      reportInteractiveRects();
    });

    return () => cancelAnimationFrame(frameId);
  });

  onMount(async () => {
    rectInterval = setInterval(reportInteractiveRects, 500);

    unlistenState = await listen<LiveGameState>("live:game-state", (event) => {
      const prevLevel = liveGameState?.level ?? 0;
      liveGameState = event.payload;

      // Auto trigger scan on milestone levels 3, 7, 11, 15
      if (
        liveGameState.is_in_game &&
        [3, 7, 11, 15].includes(liveGameState.level) &&
        liveGameState.level !== lastScannedLevel &&
        liveGameState.level > prevLevel
      ) {
        lastScannedLevel = liveGameState.level;
        void handleScanCards();
      }

      // Re-query advice if gold or level changed
      if (championId > 0 && liveGameState.is_in_game) {
        void getLiveBuildAdvice({
          championId,
          strategyId: selectedStrategyId,
          currentGold: liveGameState.current_gold,
          inventoryItemIds: liveGameState.items.map((i) => i.item_id),
          level: liveGameState.level,
          gameTime: liveGameState.game_time,
          augmentIds,
        }).then((res) => {
          liveAdvice = res;
        }).catch(() => {});
      }
    });

    unlistenSync = await listen<Record<string, unknown>>("overlay:state-sync", (event) => {
      const data = event.payload;
      if (
        data.interactiveRects !== undefined &&
        data.championId === undefined &&
        data.augmentIds === undefined &&
        data.offeredAugments === undefined &&
        data.strategyId === undefined
      ) {
        return;
      }
      if (typeof data.championId === "number" && data.championId > 0) {
        championId = data.championId;
      }
      if (typeof data.championName === "string") {
        championName = data.championName;
      }
      if (typeof data.strategyId === "string") {
        selectedStrategyId = data.strategyId;
      }
      if (Array.isArray(data.augmentIds)) {
        augmentIds = data.augmentIds as string[];
      }
      if (Array.isArray(data.offeredAugments)) {
        offeredAugments = data.offeredAugments as MayhemAugmentItem[];
      }
      if (championId > 0) {
        void loadBuildData(championId, augmentIds);
      }
    });
  });

  onDestroy(() => {
    if (rectInterval) clearInterval(rectInterval);
    unlistenState?.();
    unlistenSync?.();
  });
</script>

<div class="relative w-screen h-screen overflow-hidden select-none bg-transparent">
  <!-- Center Screen 3 Augment Cards Overlay -->
  {#if offeredAugments.length > 0}
    <!-- Sorted offered augments by WPA for ranking badges -->
    {@const sortedByWpa = [...offeredAugments].sort((a, b) => (b.wpa ?? 0) - (a.wpa ?? 0))}
    <div
      class="absolute inset-0 pointer-events-none"
      role="region"
      aria-label="Augment Card Overlay"
    >
      {#each offeredAugments as item, idx}
        {@const rankIndex = sortedByWpa.findIndex((a) => a.id === item.id)}
        {@const cardLeft = idx === 0 ? "21%" : idx === 1 ? "41.5%" : "62%"}
        <div
          class="absolute pointer-events-none flex flex-col rounded-xl border-2 bg-gray-950/95 p-3.5 backdrop-blur-md transition shadow-2xl {getTierBorderClass(item.rarity)}"
          style="left: {cardLeft}; width: 17%; top: 6%;"
        >
          <!-- Recommendation rank header -->
          <div class="flex items-center justify-between gap-1 pb-2 border-b border-gray-800">
            <span
              class="px-2 py-0.5 rounded text-[10px] font-black uppercase tracking-wider {rankIndex === 0
                ? 'bg-emerald-500/20 text-emerald-300 border border-emerald-500/40 shadow-sm'
                : 'bg-gray-800 text-gray-300 border border-gray-700'}"
            >
              {rankIndex === 0 ? "★ #1 Best Pick" : `#${rankIndex + 1} Choice`}
            </span>
            <span class="px-1.5 py-0.5 rounded text-[10px] font-bold border uppercase {getTierBadgeColor(item.rarity)}">
              {item.rarity || "Silver"}
            </span>
          </div>

          <!-- Augment Identity -->
          <div class="flex items-center gap-2.5 my-2.5">
            <div class="relative shrink-0">
              <AugmentIcon
                src={item.icon_url}
                name={item.name}
                rarity={item.rarity}
                class="h-9 w-9"
              />
            </div>
            <div class="min-w-0 flex-1">
              <div class="text-xs font-bold text-gray-100 truncate">{item.name}</div>
              <div class="text-[11px] font-semibold text-emerald-400">
                WPA {((item.wpa ?? 0) * 100) >= 0 ? "+" : ""}{((item.wpa ?? 0) * 100).toFixed(1)}%
              </div>
            </div>
          </div>

          <!-- Key Stats -->
          <div class="grid grid-cols-2 gap-1.5 py-1.5 px-2 rounded-lg bg-gray-900/80 border border-gray-800 text-[10px]">
            <div>
              <span class="text-gray-400">Win Rate:</span>
              <span class="font-bold text-gray-200 ml-1">{((item.win_rate ?? 0) * 100).toFixed(1)}%</span>
            </div>
            <div>
              <span class="text-gray-400">Games:</span>
              <span class="font-bold text-gray-300 ml-1">{item.games ?? 0}</span>
            </div>
          </div>

          <!-- Description preview -->
          {#if item.tooltip}
            <p class="text-[10px] text-gray-300 mt-2 line-clamp-3 leading-relaxed">
              {item.tooltip}
            </p>
          {/if}

          <!-- Action buttons -->
          <div class="mt-3 flex items-center gap-1.5 pointer-events-auto" data-overlay-interactive="true">
            <button
              type="button"
              onclick={() => selectAugment(item)}
              class="flex-1 py-1.5 rounded-lg bg-emerald-600 hover:bg-emerald-500 text-white font-bold text-[11px] transition shadow-md flex items-center justify-center gap-1 cursor-pointer"
            >
              <span class="icon-[heroicons--check-circle] text-xs"></span>
              Select
            </button>
            <button
              type="button"
              onclick={() => offeredAugments = []}
              class="px-2 py-1.5 rounded-lg bg-gray-800 hover:bg-gray-700 text-gray-300 text-[11px] transition cursor-pointer"
              title="Dismiss cards"
            >
              <span class="icon-[heroicons--x-mark] text-xs"></span>
            </button>
          </div>
        </div>
      {/each}
    </div>
  {/if}

  <!-- Top-Right Compact Mayhem HUD Pill & Drawer -->
  <div
    class="absolute top-3 right-3 pointer-events-auto flex flex-col items-end transition"
    role="region"
    aria-label="Mayhem In-Game HUD"
  >
    <!-- Top Pill Bar -->
    <div class="flex items-center gap-2 px-3 py-1.5 rounded-xl border border-gray-700/80 bg-gray-950/90 backdrop-blur-md shadow-xl text-xs font-semibold text-gray-200" data-overlay-interactive="true">
      <span class="flex items-center gap-1.5 font-bold text-purple-300">
        <span class="icon-[heroicons--sparkles] text-amber-400 text-sm"></span>
        {championName || liveGameState?.champion_name || "ARAM Mayhem"}
      </span>

      {#if liveGameState?.is_in_game}
        <span class="px-1.5 py-0.5 rounded bg-purple-900/60 border border-purple-500/40 text-[10px] text-purple-200 font-bold">
          Lv {liveGameState.level}
        </span>
        <span class="px-1.5 py-0.5 rounded bg-amber-950/60 border border-amber-500/40 text-[10px] text-amber-300 font-bold flex items-center gap-0.5">
          <span class="icon-[heroicons--currency-dollar] text-xs"></span>
          {liveGameState.current_gold}g
        </span>
      {/if}

      <button
        type="button"
        onclick={handleScanCards}
        disabled={scanning}
        class="flex items-center gap-1 px-2 py-1 rounded-lg bg-purple-600/80 hover:bg-purple-500 disabled:opacity-50 text-white text-[11px] font-bold shadow-sm cursor-pointer transition"
        title="Scan screen for Augment Cards (OCR)"
      >
        <span class="icon-[heroicons--camera] text-xs"></span>
        {scanning ? "Scanning..." : "Scan Cards"}
      </button>

      <button
        type="button"
        onclick={() => hudExpanded = !hudExpanded}
        class="p-1 rounded-lg hover:bg-gray-800 text-gray-300 transition cursor-pointer"
        title={hudExpanded ? "Collapse HUD" : "Expand HUD"}
      >
        <span class={hudExpanded ? "icon-[heroicons--chevron-up] text-sm" : "icon-[heroicons--chevron-down] text-sm"}></span>
      </button>
    </div>

    <!-- Expanded Advisor & Synergies Drawer -->
    {#if hudExpanded}
      <div class="mt-2 w-80 max-h-[85vh] overflow-y-auto rounded-xl border border-gray-800 bg-gray-950/95 backdrop-blur-md p-3 shadow-2xl flex flex-col gap-3 text-xs" data-overlay-interactive="true">
        <!-- Strategy Selector -->
        {#if buildSummary && buildSummary.strategies.length > 1}
          <div class="flex items-center justify-between gap-1 border-b border-gray-800/80 pb-2">
            <span class="text-[10px] font-bold text-gray-400 uppercase tracking-wide">Strategy:</span>
            <select
              bind:value={selectedStrategyId}
              onchange={() => {
                emitOverlayState({
                  championId,
                  championName,
                  strategyId: selectedStrategyId,
                  augmentIds,
                });
                loadBuildData(championId, augmentIds);
              }}
              class="bg-gray-900 border border-gray-700 text-gray-200 text-[11px] rounded px-1.5 py-0.5 font-medium cursor-pointer"
            >
              {#each buildSummary.strategies as strat}
                <option value={strat.id}>{strat.label}</option>
              {/each}
            </select>
          </div>
        {/if}

        <!-- Live Shop & Stage Advisor -->
        {#if liveAdvice}
          <div class="rounded-lg border border-purple-900/40 bg-purple-950/20 p-2.5 flex flex-col gap-2">
            <div class="flex items-center justify-between">
              <span class="text-[10px] font-black uppercase tracking-wider text-purple-300 flex items-center gap-1">
                <span class="icon-[heroicons--shopping-bag] text-xs"></span>
                {liveAdvice.stage_title || "Build Advisor"}
              </span>
              {#if liveAdvice.gold_needed > 0}
                <span class="text-[10px] text-amber-300 font-bold">
                  Need {liveAdvice.gold_needed}g
                </span>
              {/if}
            </div>

            <!-- Next item target -->
            {#if liveAdvice.next_target}
              <div class="flex items-center gap-2 p-1.5 rounded bg-gray-900/80 border border-gray-800">
                <img
                  src={liveAdvice.next_target.icon_url}
                  alt={liveAdvice.next_target.name}
                  class="w-6 h-6 rounded border border-amber-500/50 object-cover"
                />
                <div class="min-w-0 flex-1">
                  <div class="text-[11px] font-bold text-gray-100 truncate">
                    {liveAdvice.next_target.name}
                  </div>
                  <div class="text-[9px] text-gray-400">
                    Cost: {liveAdvice.next_target.price_total ?? 0}g
                  </div>
                </div>
              </div>
            {/if}

            <!-- Immediate affordable items -->
            {#if liveAdvice.immediate_purchases && liveAdvice.immediate_purchases.length > 0}
              <div>
                <span class="text-[9px] font-bold uppercase tracking-wider text-emerald-400">Can Buy Now:</span>
                <div class="flex flex-wrap gap-1 mt-1">
                  {#each liveAdvice.immediate_purchases as item}
                    <div class="flex items-center gap-1 px-1.5 py-0.5 rounded bg-emerald-950/50 border border-emerald-500/40 text-[10px] text-emerald-200">
                      <img src={item.icon_url} alt={item.name} class="w-4 h-4 rounded" />
                      <span class="font-medium truncate max-w-[90px]">{item.name}</span>
                    </div>
                  {/each}
                </div>
              </div>
            {/if}
          </div>
        {/if}

        <!-- Core Build Target Items (6 slots) -->
        {#if activeStrategy?.items && activeStrategy.items.length > 0}
          <div class="flex flex-col gap-1.5">
            <span class="text-[10px] font-bold text-gray-400 uppercase tracking-wide">Target Core Items</span>
            <div class="grid grid-cols-6 gap-1">
              {#each activeStrategy.items.slice(0, 6) as item}
                <div class="relative group aspect-square rounded border border-gray-800 bg-gray-900 p-0.5 flex items-center justify-center">
                  <img src={item.icon_url} alt={item.name} class="w-full h-full object-cover rounded" />
                </div>
              {/each}
            </div>
          </div>
        {/if}

        <!-- Active Augments Slots (4 slots) -->
        <div class="flex flex-col gap-1.5 border-t border-gray-800 pt-2">
          <div class="flex items-center justify-between">
            <span class="text-[10px] font-bold text-gray-400 uppercase tracking-wide">
              Active Augments ({augmentIds.length}/4)
            </span>
            {#if augmentIds.length > 0}
              <button
                type="button"
                onclick={() => {
                  augmentIds = [];
                  emitOverlayState({ championId, championName, strategyId: selectedStrategyId, augmentIds });
                  loadBuildData(championId, []);
                }}
                class="text-[9px] text-red-400 hover:text-red-300 font-semibold cursor-pointer"
              >
                Clear
              </button>
            {/if}
          </div>

          <div class="grid grid-cols-4 gap-1.5">
            {#each Array(4) as _, idx}
              {@const augName = augmentIds[idx]}
              {@const augItem = activeStrategy?.augments?.find((a) => a.name === augName)}
              {#if augName}
                <div class="relative group aspect-square rounded-lg border border-purple-500/40 bg-purple-950/40 p-1 flex flex-col items-center justify-center">
                  <AugmentIcon
                    src={augItem?.icon_url}
                    name={augName}
                    rarity={augItem?.rarity}
                    class="h-7 w-7"
                  />
                  <button
                    type="button"
                    onclick={() => removeAugment(augName)}
                    class="absolute -top-1 -right-1 w-4 h-4 rounded-full bg-red-600 hover:bg-red-500 text-white text-[10px] flex items-center justify-center opacity-0 group-hover:opacity-100 transition cursor-pointer"
                  >
                    ×
                  </button>
                </div>
              {:else}
                <div class="aspect-square rounded-lg border border-dashed border-gray-800 bg-gray-900/40 flex items-center justify-center text-gray-600 text-[10px] font-bold">
                  {idx + 1}
                </div>
              {/if}
            {/each}
          </div>
        </div>

        <!-- Top Mined Synergies / Combos -->
        {#if topCombos.length > 0}
          <div class="flex flex-col gap-1.5 border-t border-gray-800 pt-2">
            <span class="text-[10px] font-bold text-gray-400 uppercase tracking-wide flex items-center gap-1">
              <span class="icon-[heroicons--fire] text-amber-400 text-xs"></span>
              Top Mined Combos
            </span>
            <div class="flex flex-col gap-1">
              {#each topCombos.slice(0, 3) as combo}
                <div class="flex items-center justify-between p-1.5 rounded bg-gray-900/60 border border-gray-800/80 text-[10px]">
                  <span class="font-medium text-gray-300 truncate max-w-[170px]">
                    {combo.name}
                  </span>
                  <span class="font-bold text-emerald-400 shrink-0">
                    +{((combo.wpa ?? 0) * 100).toFixed(1)}%
                  </span>
                </div>
              {/each}
            </div>
          </div>
        {/if}
      </div>
    {/if}
  </div>
</div>
