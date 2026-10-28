<script lang="ts">
  import type { LiveGameState, MayhemBuildStrategy, MayhemLiveAdvice } from "../../types/champ_select";
  import GameEntityPopover from "../common/GameEntityPopover.svelte";

  let {
    liveGameState,
    liveAdvice = null,
    activeStrategy = null,
  }: {
    liveGameState: LiveGameState;
    liveAdvice?: MayhemLiveAdvice | null;
    activeStrategy?: MayhemBuildStrategy | null;
  } = $props();

  const formattedTime = $derived.by(() => {
    const totalSec = Math.floor(liveGameState.game_time);
    const m = Math.floor(totalSec / 60);
    const s = totalSec % 60;
    return `${m.toString().padStart(2, "0")}:${s.toString().padStart(2, "0")}`;
  });

  const nextTarget = $derived(liveAdvice?.next_target ?? null);
  const goldNeeded = $derived(liveAdvice?.gold_needed ?? 0);
  const isTargetAffordable = $derived(nextTarget ? liveGameState.current_gold >= nextTarget.price_total : false);
  const targetProgress = $derived.by(() => {
    if (!nextTarget || nextTarget.price_total <= 0) return 100;
    return Math.min(100, Math.round((liveGameState.current_gold / nextTarget.price_total) * 100));
  });
</script>

<div class="relative overflow-hidden rounded-2xl border border-emerald-500/40 bg-gradient-to-r from-emerald-950/40 via-gray-900/90 to-purple-950/30 p-4 shadow-xl backdrop-blur-md">
  <!-- Glowing accent border line -->
  <div class="absolute inset-x-0 top-0 h-0.5 bg-gradient-to-r from-emerald-400 via-teal-300 to-purple-500"></div>

  <div class="flex flex-col lg:flex-row items-stretch lg:items-center justify-between gap-4">
    <!-- Left: Match & Player Snapshot -->
    <div class="flex items-center gap-3.5">
      <div class="relative flex items-center justify-center w-12 h-12 rounded-xl bg-gray-950/80 border border-emerald-500/50 shadow-inner">
        <span class="icon-[heroicons--shopping-bag] text-2xl text-emerald-400"></span>
        <span class="absolute -top-1 -right-1 flex h-3 w-3">
          <span class="animate-ping absolute inline-flex h-full w-full rounded-full bg-emerald-400 opacity-75"></span>
          <span class="relative inline-flex rounded-full h-3 w-3 bg-emerald-500"></span>
        </span>
      </div>

      <div>
        <div class="flex items-center gap-2">
          <span class="text-xs font-bold uppercase tracking-wider text-emerald-400">Live Match Active</span>
          <span class="text-gray-600">|</span>
          <span class="text-xs font-mono text-gray-300 font-semibold">{formattedTime}</span>
          <span class="px-1.5 py-0.5 rounded text-[10px] font-bold bg-emerald-500/20 text-emerald-300 border border-emerald-500/30">
            Lv {liveGameState.level}
          </span>
        </div>
        <div class="flex items-baseline gap-2 mt-0.5">
          <span class="text-base font-extrabold text-white">{liveGameState.champion_name}</span>
          <span class="text-xs text-amber-300 font-bold font-mono">
            💰 {Math.floor(liveGameState.current_gold).toLocaleString()} gold
          </span>
        </div>
      </div>
    </div>

    <!-- Center: Next Power Spike Target -->
    {#if nextTarget}
      {@const targetId = `live-target-${nextTarget.id}`}
      <div class="flex-1 max-w-md bg-gray-950/60 rounded-xl border border-gray-800 p-2.5 flex items-center gap-3">
        <div id={targetId} class="relative flex-shrink-0 cursor-help">
          {#if nextTarget.icon_url}
            <img src={nextTarget.icon_url} alt={nextTarget.name} class="w-10 h-10 rounded-lg object-cover border border-amber-400/50 shadow" />
          {:else}
            <div class="w-10 h-10 rounded-lg bg-gray-800 flex items-center justify-center text-xs font-bold text-gray-300">
              {nextTarget.name.slice(0, 2)}
            </div>
          {/if}
          {#if isTargetAffordable}
            <span class="absolute -top-1 -right-1 flex h-2.5 w-2.5 rounded-full bg-emerald-400 ring-2 ring-gray-950"></span>
          {/if}
        </div>

        <GameEntityPopover
          {targetId}
          name={nextTarget.name}
          iconUrl={nextTarget.icon_url}
          description={nextTarget.description}
          itemClass={nextTarget.item_class}
          priceTotal={nextTarget.price_total}
          placement="top"
        />

        <div class="flex-1 min-w-0">
          <div class="flex items-center justify-between text-xs">
            <span class="font-bold text-gray-200 truncate">{nextTarget.name}</span>
            <span class="font-mono text-amber-300 text-[11px] font-bold">{nextTarget.price_total}g</span>
          </div>

          <!-- Progress Bar -->
          <div class="w-full bg-gray-800 rounded-full h-1.5 mt-1.5 overflow-hidden">
            <div
              class="h-1.5 rounded-full transition-all duration-300 {isTargetAffordable ? 'bg-emerald-400 shadow-[0_0_8px_rgba(52,211,153,0.5)]' : 'bg-gradient-to-r from-amber-500 to-amber-300'}"
              style="width: {targetProgress}%"
            ></div>
          </div>

          <div class="flex items-center justify-between text-[10px] mt-1 font-mono">
            {#if isTargetAffordable}
              <span class="text-emerald-300 font-extrabold flex items-center gap-1">
                <span class="icon-[heroicons--check-circle] text-xs"></span> Ready to Purchase!
              </span>
              <span class="text-gray-400">100%</span>
            {:else}
              <span class="text-gray-400">Need +{goldNeeded.toLocaleString()}g</span>
              <span class="text-amber-400 font-semibold">{targetProgress}%</span>
            {/if}
          </div>
        </div>
      </div>
    {:else}
      <div class="text-xs text-gray-400 italic bg-gray-950/40 px-3 py-2 rounded-xl border border-gray-800">
        Full build completed or no remaining target item.
      </div>
    {/if}

    <!-- Right: Stage badge & Quick Actions -->
    <div class="flex flex-wrap items-center gap-2">
      {#if activeStrategy?.label}
        <div class="px-2.5 py-1 rounded-lg border border-gray-800 bg-gray-950/60 text-[11px] text-gray-300 font-medium">
          {activeStrategy.label}
        </div>
      {/if}
      {#if liveAdvice?.stage_title}
        <div class="px-3 py-1.5 rounded-xl border border-purple-500/40 bg-purple-950/30 text-xs font-semibold text-purple-200">
          {liveAdvice.stage_title}
        </div>
      {/if}
    </div>
  </div>

  <!-- Immediate Affordable Purchases row -->
  {#if liveAdvice?.immediate_purchases && liveAdvice.immediate_purchases.length > 0}
    <div class="mt-3 pt-3 border-t border-gray-800/80 flex flex-wrap items-center gap-2">
      <span class="text-[11px] font-bold text-gray-400 uppercase tracking-wide flex items-center gap-1">
        <span class="icon-[heroicons--sparkles] text-amber-400 text-xs"></span>
        Affordable Now:
      </span>
      {#each liveAdvice.immediate_purchases.slice(0, 6) as affItem}
        {@const affId = `aff-it-${affItem.id}`}
        <div
          id={affId}
          class="flex items-center gap-1.5 px-2 py-1 rounded-lg border border-emerald-500/30 bg-emerald-950/20 hover:border-emerald-400/60 transition cursor-help"
        >
          {#if affItem.icon_url}
            <img src={affItem.icon_url} alt={affItem.name} class="w-5 h-5 rounded object-cover" />
          {/if}
          <span class="text-xs font-semibold text-gray-200">{affItem.name}</span>
          <span class="text-[10px] font-mono text-amber-300 font-bold">{affItem.price_total}g</span>

          <GameEntityPopover
            targetId={affId}
            name={affItem.name}
            iconUrl={affItem.icon_url}
            description={affItem.description}
            priceTotal={affItem.price_total}
            placement="top"
          />
        </div>
      {/each}
    </div>
  {/if}

  <!-- Current Inventory display -->
  {#if liveGameState.items && liveGameState.items.length > 0}
    <div class="mt-2.5 pt-2 border-t border-gray-800/60 flex items-center gap-1.5">
      <span class="text-[10px] font-semibold text-gray-400 uppercase tracking-wider">Inventory ({liveGameState.items.length}/6):</span>
      <div class="flex items-center gap-1.5">
        {#each liveGameState.items as inv}
          <div class="px-1.5 py-0.5 rounded bg-gray-800/80 border border-gray-700 text-[10px] font-mono text-gray-300 flex items-center gap-1" title={inv.display_name}>
            <span>{inv.display_name}</span>
            {#if inv.count > 1}
              <span class="text-amber-400 font-bold">x{inv.count}</span>
            {/if}
          </div>
        {/each}
      </div>
    </div>
  {/if}
</div>
