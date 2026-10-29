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
    MayhemChampionBuildSummary,
    MayhemLiveAdvice,
    OverlayAugmentCard,
  } from "../../types/champ_select";
  import AugmentIcon from "../common/AugmentIcon.svelte";
  import { logger } from "../../utils/logger";

  let liveGameState = $state<LiveGameState | null>(null);
  let championId = $state<number>(0);
  let championName = $state<string>("");
  let selectedStrategyId = $state<string>("highest_wr");
  let augmentIds = $state<string[]>([]);
  let offeredAugments = $state<OverlayAugmentCard[]>([]);
  let buildSummary = $state<MayhemChampionBuildSummary | null>(null);
  let liveAdvice = $state<MayhemLiveAdvice | null>(null);
  let topCombos = $state<ServerStatisticRow[]>([]);
  let scanning = $state<boolean>(false);
  let hudExpanded = $state<boolean>(true);
  let cardCanvas: HTMLCanvasElement | null = $state(null);

  let unlistenState: UnlistenFn | null = null;
  let unlistenSync: UnlistenFn | null = null;
  let unlistenCards: UnlistenFn | null = null;
  let unlistenScanStatus: UnlistenFn | null = null;

  const activeStrategy = $derived(
    buildSummary?.strategies.find((s) => s.id === selectedStrategyId) ??
      buildSummary?.strategies[0] ??
      null
  );


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

  $effect(() => {
    if (!cardCanvas) return;
    const canvas = cardCanvas;
    const ctx = canvas.getContext("2d");
    if (!ctx) return;

    const W = window.innerWidth;
    const H = window.innerHeight;
    canvas.width = W;
    canvas.height = H;

    ctx.clearRect(0, 0, W, H);
    if (!offeredAugments || offeredAugments.length === 0) {
      return;
    }

    const sortedByWpa = [...offeredAugments].sort((a, b) => (b.wpa ?? 0) - (a.wpa ?? 0));

    for (const card of offeredAugments) {
      const rankIdx = sortedByWpa.findIndex((c) => c.id === card.id);
      const isBest = rankIdx === 0;

      const slotNorm = card.slot === 0 ? 0.21 : card.slot === 1 ? 0.415 : 0.62;
      const x = Math.round(slotNorm * W);
      const y = Math.round(0.08 * H);
      const w = Math.round(0.17 * W);
      const h = Math.round(0.24 * H);
      const radius = 10;

      ctx.save();
      ctx.beginPath();
      ctx.roundRect(x, y, w, h, radius);
      ctx.fillStyle = "#090d16f2";
      ctx.fill();

      const rarity = (card.rarity || "").toLowerCase();
      let borderColor = "#a3b9bd";
      if (rarity.includes("prismatic")) {
        borderColor = "#e78fff";
      } else if (rarity.includes("gold")) {
        borderColor = "#e3c49f";
      }
      ctx.strokeStyle = borderColor;
      ctx.lineWidth = 2;
      ctx.stroke();

      const p = 12;
      let curY = y + p + 12;

      ctx.font = "bold 11px sans-serif";
      if (isBest) {
        ctx.fillStyle = "#34d399";
        ctx.fillText("★ #1 BEST", x + p, curY);
      } else {
        ctx.fillStyle = "#9ca3af";
        ctx.fillText(rankIdx === 1 ? "#2 CHOICE" : "#3 CHOICE", x + p, curY);
      }

      ctx.textAlign = "right";
      ctx.fillStyle = borderColor;
      ctx.fillText((card.rarity || "SILVER").toUpperCase(), x + w - p, curY);
      ctx.textAlign = "left";

      curY += 20;
      ctx.fillStyle = "#f3f4f6";
      ctx.font = "bold 14px sans-serif";
      let nameText = card.name || "Unknown Augment";
      if (ctx.measureText(nameText).width > w - p * 2) {
        while (nameText.length > 3 && ctx.measureText(nameText + "…").width > w - p * 2) {
          nameText = nameText.slice(0, -1);
        }
        nameText += "…";
      }
      ctx.fillText(nameText, x + p, curY);

      curY += 18;
      ctx.font = "bold 12px sans-serif";
      ctx.fillStyle = "#10b981";
      const wrText = `WR ${(card.win_rate * 100).toFixed(1)}%`;
      ctx.fillText(wrText, x + p, curY);

      const wpaVal = card.wpa ?? 0;
      const wpaSign = wpaVal >= 0 ? "+" : "";
      const wpaText = `${wpaSign}${(wpaVal * 100).toFixed(1)}% WPA`;
      ctx.fillStyle = wpaVal >= 0 ? "#34d399" : "#f87171";
      ctx.fillText(wpaText, x + p + 85, curY);

      ctx.textAlign = "right";
      ctx.font = "11px sans-serif";
      ctx.fillStyle = "#9ca3af";
      ctx.fillText(`${card.games ?? 0}g`, x + w - p, curY);
      ctx.textAlign = "left";

      curY += 10;
      ctx.strokeStyle = "#1f2937";
      ctx.lineWidth = 1;
      ctx.beginPath();
      ctx.moveTo(x + p, curY);
      ctx.lineTo(x + w - p, curY);
      ctx.stroke();

      curY += 14;
      ctx.font = "bold 9px sans-serif";
      ctx.fillStyle = "#9ca3af";
      ctx.fillText("TOP COMBOS & SYNERGIES", x + p, curY);

      curY += 14;
      ctx.font = "10px sans-serif";
      if (card.combos && card.combos.length > 0) {
        for (const combo of card.combos.slice(0, 3)) {
          ctx.fillStyle = "#d1d5db";
          let label = combo.label || "";
          const wrStr = `${(combo.win_rate * 100).toFixed(1)}% WR`;
          const wrW = ctx.measureText(wrStr).width;
          const maxLabelW = w - p * 2 - wrW - 8;

          if (ctx.measureText(label).width > maxLabelW) {
            while (label.length > 3 && ctx.measureText(label + "…").width > maxLabelW) {
              label = label.slice(0, -1);
            }
            label += "…";
          }
          ctx.fillText(label, x + p, curY);

          ctx.textAlign = "right";
          ctx.fillStyle = "#34d399";
          ctx.fillText(wrStr, x + w - p, curY);
          ctx.textAlign = "left";
          curY += 13;
        }
      } else if (card.synergy_items && card.synergy_items.length > 0) {
        let itemsStr = card.synergy_items.map((i) => i.name).join(" + ");
        if (ctx.measureText(itemsStr).width > w - p * 2) {
          while (itemsStr.length > 3 && ctx.measureText(itemsStr + "…").width > w - p * 2) {
            itemsStr = itemsStr.slice(0, -1);
          }
          itemsStr += "…";
        }
        ctx.fillStyle = "#d1d5db";
        ctx.fillText(itemsStr, x + p, curY);
      } else {
        ctx.fillStyle = "#6b7280";
        ctx.fillText("No combo data available", x + p, curY);
      }

      ctx.restore();
    }
  });

  onMount(async () => {
    unlistenState = await listen<LiveGameState>("live:game-state", (event) => {
      liveGameState = event.payload;

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

    unlistenCards = await listen<OverlayAugmentCard[]>("overlay:cards-updated", (event) => {
      offeredAugments = event.payload ?? [];
    });

    unlistenScanStatus = await listen<{ scanning: boolean }>("overlay:scan-status", (event) => {
      scanning = Boolean(event.payload?.scanning);
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
        offeredAugments = data.offeredAugments as OverlayAugmentCard[];
      }
      if (championId > 0) {
        void loadBuildData(championId, augmentIds);
      }
    });
  });

  onDestroy(() => {
    unlistenState?.();
    unlistenSync?.();
    unlistenCards?.();
    unlistenScanStatus?.();
  });
</script>

<div class="relative w-screen h-screen overflow-hidden select-none bg-transparent">
  <canvas bind:this={cardCanvas} class="absolute inset-0 w-full h-full pointer-events-none" style="z-index: 40;"></canvas>

  <!-- Top-Right Compact Mayhem HUD Pill & Drawer -->
  <div
    class="absolute top-3 right-3 pointer-events-auto flex flex-col items-end transition"
    role="region"
    aria-label="Mayhem In-Game HUD"
  >
    <!-- Top Pill Bar -->
    <div class="flex items-center gap-2 px-3 py-1.5 rounded-xl border border-gray-700/80 bg-[#090d16]/95 shadow-xl text-xs font-semibold text-gray-200" data-overlay-interactive="true">
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

      <div class="flex items-center gap-1.5 px-2 py-0.5 rounded-lg bg-purple-950/90 border border-purple-500/40 text-[10px] font-bold text-purple-200">
        <kbd class="px-1 py-0.5 rounded bg-purple-800 text-white font-mono text-[9px]">F8</kbd>
        <span>{scanning ? "Scanning..." : offeredAugments.length > 0 ? "Rescan Cards" : "Scan Cards"}</span>
        {#if offeredAugments.length > 0}
          <kbd class="ml-1 px-1 py-0.5 rounded bg-gray-800 text-gray-300 font-mono text-[9px]">F7</kbd>
          <span class="text-gray-400">Hide</span>
        {/if}
      </div>

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
      <div class="mt-2 w-80 max-h-[85vh] overflow-y-auto rounded-xl border border-gray-800 bg-[#090d16]/95 p-3 shadow-2xl flex flex-col gap-3 text-xs" data-overlay-interactive="true">
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
