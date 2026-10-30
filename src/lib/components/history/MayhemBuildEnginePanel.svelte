<script lang="ts">
  import type { BuildRecommendationResult, MayhemBuildItem, MayhemBuildAugment } from "../../types/champ_select";
  import GameEntityPopover from "../common/GameEntityPopover.svelte";
  import AugmentIcon from "../common/AugmentIcon.svelte";

  let {
    recommendation = null,
    loading = false,
    knownItemMap = new Map<number, MayhemBuildItem>(),
    knownAugmentMap = new Map<string, MayhemBuildAugment>(),
    onSelectItem,
  }: {
    recommendation: BuildRecommendationResult | null;
    loading?: boolean;
    knownItemMap?: Map<number, MayhemBuildItem>;
    knownAugmentMap?: Map<string, MayhemBuildAugment>;
    onSelectItem?: (item: MayhemBuildItem) => void;
  } = $props();

  function getItemIconUrl(itemId: number): string {
    return `https://raw.communitydragon.org/latest/plugins/rcp-be-lol-game-data/global/default/assets/items/icons2d/${itemId}.png`.toLowerCase();
  }

  function formatPct(val: number | undefined): string {
    if (val === undefined || Number.isNaN(val)) return "0.0%";
    const pct = val > 1 ? val : val * 100;
    return `${pct.toFixed(1)}%`;
  }

  function formatWpa(val: number | undefined): string {
    if (val === undefined || Number.isNaN(val)) return "+0.0%";
    const pct = Math.abs(val) <= 1 ? val * 100 : val;
    return `${pct >= 0 ? "+" : ""}${pct.toFixed(1)}%`;
  }

  function handleItemClick(itemId: number) {
    if (!onSelectItem) return;
    const known = knownItemMap.get(itemId);
    const item: MayhemBuildItem = known ?? {
      id: itemId,
      name: `Item #${itemId}`,
      icon_url: getItemIconUrl(itemId),
      price_total: 0,
    };
    onSelectItem(item);
  }
</script>

<div class="rounded-xl border border-gray-800 bg-gray-900/80 p-4 shadow-xl backdrop-blur-sm space-y-3">
  <!-- Header with Live Recommendations & Engine Stats -->
  <div class="flex flex-wrap items-center justify-between gap-2 border-b border-gray-800 pb-2.5">
    <div class="flex items-center gap-2">
      <div class="flex h-7 w-7 items-center justify-center rounded-lg bg-cyan-500/10 text-cyan-400 border border-cyan-500/30">
        <span class="icon-[heroicons--cpu-chip] text-sm"></span>
      </div>
      <div>
        <h3 class="text-xs font-bold uppercase tracking-wider text-gray-200">
          Mayhem Build Engine Recommendations
        </h3>
        <p class="text-[10px] text-gray-400">
          Empirical bitset synergy optimization powered by sub-millisecond mining
        </p>
      </div>
    </div>

    {#if recommendation?.stats}
      {@const stats = recommendation.stats}
      <div class="flex items-center gap-2 text-[10px] font-mono">
        <span class="rounded bg-gray-800/80 px-2 py-0.5 text-gray-300 border border-gray-700/60" title="Total engine computation time">
          <span class="text-gray-400">Time:</span> {stats.total_time_ms.toFixed(1)}ms
        </span>
        <span class="rounded bg-gray-800/80 px-2 py-0.5 text-gray-300 border border-gray-700/60" title="Dataset match count evaluated">
          <span class="text-gray-400">Matches:</span> {stats.match_count.toLocaleString()}
        </span>
        {#if stats.cache_hit}
          <span class="rounded bg-emerald-950/70 border border-emerald-800/60 px-1.5 py-0.5 text-[9px] font-bold text-emerald-400">
            CACHE HIT
          </span>
        {/if}
      </div>
    {/if}
  </div>

  {#if loading}
    <div class="flex h-36 items-center justify-center gap-2 text-xs text-gray-400">
      <span class="icon-[heroicons--arrow-path] text-base animate-spin text-cyan-400"></span>
      <span>Calculating optimal builds from match synergies...</span>
    </div>
  {:else if !recommendation}
    <div class="flex h-24 items-center justify-center text-xs text-gray-500 italic">
      Select a champion to generate build engine recommendations
    </div>
  {:else}
    <!-- 3-Column Recommendations: Next Items, Target 6-Item Build, Recommended Boots -->
    <div class="grid grid-cols-1 md:grid-cols-3 gap-3">
      <!-- 1. Next Priority Items (Legendaries) -->
      <div class="rounded-xl border border-gray-800/90 bg-gray-950/40 p-3 flex flex-col justify-between">
        <div>
          <div class="flex items-center justify-between mb-2">
            <span class="text-[10px] font-bold uppercase tracking-wider text-amber-400 flex items-center gap-1">
              <span class="icon-[heroicons--sparkles] text-xs"></span>
              Next Priority Items
            </span>
            <span class="text-[9px] text-gray-500">Highest Lift</span>
          </div>

          {#if recommendation.next_items && recommendation.next_items.length > 0}
            <div class="space-y-2">
              {#each recommendation.next_items.slice(0, 2) as item, idx (item.item_id)}
                {@const recItem = knownItemMap.get(item.item_id)}
                <div class="flex items-center gap-2.5">
                  <div
                    id={`engine-panel-next-${item.item_id}-${idx}`}
                    role="button"
                    tabindex="0"
                    onclick={() => handleItemClick(item.item_id)}
                    onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') handleItemClick(item.item_id); }}
                    class="relative shrink-0 cursor-pointer group"
                  >
                    <img
                      src={recItem?.icon_url || getItemIconUrl(item.item_id)}
                      alt={recItem?.name || `Item #${item.item_id}`}
                      class="h-9 w-9 rounded-lg border border-amber-500/40 bg-gray-950 object-cover group-hover:border-amber-400 transition-colors"
                    />
                    <span class="absolute -top-1 -left-1 flex h-4 w-4 items-center justify-center rounded-full bg-amber-500 text-[8px] font-bold text-gray-950">
                      {idx + 1}
                    </span>
                  </div>
                  <GameEntityPopover
                    targetId={`engine-panel-next-${item.item_id}-${idx}`}
                    name={recItem?.name || `Item #${item.item_id}`}
                    iconUrl={recItem?.icon_url || getItemIconUrl(item.item_id)}
                    description={recItem?.description || item.reason}
                    priceTotal={recItem?.price_total}
                    itemClass={recItem?.item_class}
                    placement="top"
                  />
                  <div class="flex flex-col min-w-0 flex-1">
                    <span class="text-xs font-semibold text-gray-200 truncate">
                      {recItem?.name || `Item #${item.item_id}`}
                    </span>
                    <div class="flex items-center gap-1.5 mt-0.5">
                      <span class="text-[10px] font-mono text-gray-300">
                        {formatPct(item.win_rate)} WR
                      </span>
                      <span class="rounded px-1.5 py-0.2 text-[9px] font-mono font-bold {item.lift >= 0 ? 'bg-emerald-950/80 text-emerald-400 border border-emerald-800/50' : 'bg-rose-950/80 text-rose-400 border border-rose-800/50'}">
                        {formatWpa(item.lift)} Lift
                      </span>
                      {#if item.sample_size}
                        <span class="text-[9px] text-gray-500 font-mono">({item.sample_size}g)</span>
                      {/if}
                    </div>
                  </div>
                </div>
              {/each}
            </div>
          {:else}
            <div class="flex h-14 items-center justify-center text-[11px] text-gray-500 italic">
              Build full or waiting for inventory state
            </div>
          {/if}
        </div>
      </div>

      <!-- 2. Completed 5-Item Target Build (Full Width) -->
      <div class="rounded-xl border border-gray-800/90 bg-gray-950/40 p-3 flex flex-col justify-between">
        <div>
          <div class="flex items-center justify-between mb-2">
            <span class="text-[10px] font-bold uppercase tracking-wider text-purple-400 flex items-center gap-1">
              <span class="icon-[heroicons--square-3-stack-3d] text-xs"></span>
              Full 5-Item Build Path
            </span>
            <span class="text-[9px] text-gray-500 font-mono">
              {recommendation.completed_build.length}/5
            </span>
          </div>

          {#if recommendation.completed_build && recommendation.completed_build.length > 0}
            <div class="grid grid-cols-5 gap-2 w-full pt-1 pb-0.5">
              {#each recommendation.completed_build as recEntry, idx (`${recEntry.item_id}-${idx}`)}
                {@const cItem = knownItemMap.get(recEntry.item_id)}
                <div class="flex flex-col items-center group min-w-0">
                  <!-- Win Rate on Top -->
                  <div class="mb-1 text-[10px] font-mono font-bold text-purple-300 leading-none">
                    {formatPct(recEntry.win_rate)}
                  </div>

                  <div
                    id={`engine-panel-comp-${recEntry.item_id}-${idx}`}
                    role="button"
                    tabindex="0"
                    onclick={() => handleItemClick(recEntry.item_id)}
                    onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') handleItemClick(recEntry.item_id); }}
                    class="relative cursor-pointer shrink-0"
                    title={cItem?.name || `Item #${recEntry.item_id}`}
                  >
                    <img
                      src={cItem?.icon_url || getItemIconUrl(recEntry.item_id)}
                      alt={cItem?.name || `Item #${recEntry.item_id}`}
                      class="h-11 w-11 rounded-lg border border-purple-500/40 bg-gray-950 object-cover group-hover:border-purple-400 transition-colors shadow-sm"
                    />
                    <span class="absolute -bottom-1 -right-1 flex h-4 w-4 items-center justify-center rounded-full bg-gray-900 border border-gray-700 text-[8px] font-bold text-gray-300">
                      {idx + 1}
                    </span>
                  </div>
                  <GameEntityPopover
                    targetId={`engine-panel-comp-${recEntry.item_id}-${idx}`}
                    name={cItem?.name || `Item #${recEntry.item_id}`}
                    iconUrl={cItem?.icon_url || getItemIconUrl(recEntry.item_id)}
                    description={cItem?.description || recEntry.reason}
                    priceTotal={cItem?.price_total}
                    itemClass={cItem?.item_class}
                    placement="top"
                  />
                  <!-- Name & Games Count on Bottom -->
                  <div class="mt-1 flex flex-col items-center text-center w-full px-0.5 min-w-0">
                    <span class="text-[10px] font-medium text-gray-300 truncate w-full" title={cItem?.name || `Item #${recEntry.item_id}`}>
                      {cItem?.name || `Item #${recEntry.item_id}`}
                    </span>
                    {#if recEntry.sample_size}
                      <span class="text-[9px] font-mono text-gray-400 mt-0.5">
                        {recEntry.sample_size}g
                      </span>
                    {/if}
                    {#if recEntry.lift && Math.abs(recEntry.lift) > 0.001}
                      <span class="text-[8px] font-mono mt-0.5 {recEntry.lift >= 0 ? 'text-emerald-400' : 'text-rose-400'}">
                        {formatWpa(recEntry.lift)}
                      </span>
                    {/if}
                  </div>
                </div>
              {/each}
            </div>
          {:else}
            <div class="flex h-14 items-center justify-center text-[11px] text-gray-500 italic">
              Calculating full 5-item synergy path...
            </div>
          {/if}
        </div>
      </div>

      <!-- 3. Recommended Boots -->
      <div class="rounded-xl border border-gray-800/90 bg-gray-950/40 p-3 flex flex-col justify-between">
        <div>
          <div class="flex items-center justify-between mb-2">
            <span class="text-[10px] font-bold uppercase tracking-wider text-cyan-400 flex items-center gap-1">
              <span class="icon-[heroicons--bolt] text-xs"></span>
              Recommended Boots
            </span>
          </div>

          {#if recommendation.recommended_boots}
            {@const boot = recommendation.recommended_boots}
            {@const bootItem = knownItemMap.get(boot.item_id)}
            <div class="flex items-center gap-3">
              <div
                id={`engine-panel-boot-${boot.item_id}`}
                role="button"
                tabindex="0"
                onclick={() => handleItemClick(boot.item_id)}
                onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') handleItemClick(boot.item_id); }}
                class="relative shrink-0 cursor-pointer group"
              >
                <img
                  src={bootItem?.icon_url || getItemIconUrl(boot.item_id)}
                  alt={bootItem?.name || `Boot #${boot.item_id}`}
                  class="h-11 w-11 rounded-lg border border-cyan-500/40 bg-gray-950 object-cover shadow-sm group-hover:border-cyan-400 transition-colors"
                />
              </div>
              <GameEntityPopover
                targetId={`engine-panel-boot-${boot.item_id}`}
                name={bootItem?.name || `Item #${boot.item_id}`}
                iconUrl={bootItem?.icon_url || getItemIconUrl(boot.item_id)}
                description={bootItem?.description || boot.reason}
                priceTotal={bootItem?.price_total}
                itemClass={bootItem?.item_class}
                placement="top"
              />
              <div class="flex flex-col min-w-0">
                <span class="text-xs font-semibold text-gray-100 truncate">
                  {bootItem?.name || `Item #${boot.item_id}`}
                </span>
                <div class="flex items-center gap-1.5 mt-0.5">
                  <span class="text-[10px] font-mono font-bold text-gray-300">
                    {formatPct(boot.win_rate)} WR
                  </span>
                  <span class="rounded px-1.5 py-0.2 text-[9px] font-mono font-bold {boot.lift >= 0 ? 'bg-emerald-950/80 text-emerald-400 border border-emerald-800/50' : 'bg-rose-950/80 text-rose-400 border border-rose-800/50'}">
                    {formatWpa(boot.lift)} Lift
                  </span>
                </div>
                {#if boot.sample_size}
                  <span class="text-[9px] text-gray-500 font-mono">
                    {boot.sample_size.toLocaleString()} games
                  </span>
                {/if}
                {#if boot.reason}
                  <span class="text-[9px] text-gray-400 truncate mt-0.5" title={boot.reason}>
                    {boot.reason}
                  </span>
                {/if}
              </div>
            </div>
          {:else}
            <div class="flex h-14 items-center justify-center text-[11px] text-gray-500 italic">
              Boots equipped or not needed
            </div>
          {/if}
        </div>
      </div>
    </div>

    <!-- Sibling: Alternative Items Not in This Build Path -->
    {#if recommendation.alternative_items && recommendation.alternative_items.length > 0}
      <div class="pt-2.5 border-t border-gray-800/80">
        <div class="flex items-center justify-between mb-2">
          <span class="text-[10px] font-bold uppercase tracking-wider text-indigo-400 flex items-center gap-1">
            <span class="icon-[heroicons--arrows-right-left] text-xs"></span>
            Alternative Options
          </span>
          <span class="text-[9px] text-gray-500 font-mono">
            Ranked Candidate Legendaries
          </span>
        </div>

        <div class="grid grid-cols-2 sm:grid-cols-3 lg:grid-cols-4 gap-2">
          {#each recommendation.alternative_items as alt, aIdx (`${alt.item_id}-${aIdx}`)}
            {@const aItem = knownItemMap.get(alt.item_id)}
            <div
              id={`engine-panel-alt-${alt.item_id}-${aIdx}`}
              role="button"
              tabindex="0"
              onclick={() => handleItemClick(alt.item_id)}
              onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') handleItemClick(alt.item_id); }}
              class="flex items-center gap-2 rounded-lg border border-gray-800/90 bg-gray-950/60 p-2 hover:border-indigo-500/50 hover:bg-indigo-950/20 transition-all cursor-pointer group shadow-sm min-w-0"
              title={`Click to add ${aItem?.name || `Item #${alt.item_id}`} to simulator`}
            >
              <div class="relative shrink-0">
                <img
                  src={aItem?.icon_url || getItemIconUrl(alt.item_id)}
                  alt={aItem?.name || `Item #${alt.item_id}`}
                  class="h-9 w-9 rounded-lg border border-gray-800 bg-gray-950 object-cover group-hover:border-indigo-400 transition-colors"
                />
              </div>

              <GameEntityPopover
                targetId={`engine-panel-alt-${alt.item_id}-${aIdx}`}
                name={aItem?.name || `Item #${alt.item_id}`}
                iconUrl={aItem?.icon_url || getItemIconUrl(alt.item_id)}
                description={aItem?.description || alt.reason}
                priceTotal={aItem?.price_total}
                itemClass={aItem?.item_class}
                placement="top"
              />

              <div class="flex flex-col min-w-0 flex-1">
                <span class="text-[11px] font-semibold text-gray-200 truncate group-hover:text-indigo-200 transition-colors">
                  {aItem?.name || `Item #${alt.item_id}`}
                </span>
                <div class="flex items-center gap-1.5 mt-0.5">
                  <span class="text-[10px] font-mono text-gray-300">
                    {formatPct(alt.win_rate)} WR
                  </span>
                  <span class="rounded px-1 py-0.2 text-[8px] font-mono font-bold {alt.lift >= 0 ? 'bg-emerald-950/80 text-emerald-400 border border-emerald-800/50' : 'bg-rose-950/80 text-rose-400 border border-rose-800/50'}">
                    {formatWpa(alt.lift)}
                  </span>
                </div>
                {#if alt.reason}
                  <span class="text-[8px] font-mono text-gray-400 truncate mt-0.5" title={alt.reason}>
                    {alt.reason}
                  </span>
                {/if}
              </div>
            </div>
          {/each}
        </div>
      </div>
    {/if}

    <!-- Active Combos & Synergy Pairs from Engine -->
    {#if recommendation.active_combos && recommendation.active_combos.length > 0}
      <div class="pt-2 border-t border-gray-800">
        <div class="flex items-center justify-between mb-2">
          <span class="text-[10px] font-bold uppercase tracking-wider text-emerald-400 flex items-center gap-1">
            <span class="icon-[heroicons--link] text-xs"></span>
            Active Combos & Synergy Pairs ({recommendation.active_combos.length})
          </span>
          <span class="text-[9px] text-gray-500">Augment & Item Synergy</span>
        </div>

        <div class="flex flex-wrap gap-2">
          {#each recommendation.active_combos as combo, cIdx (`${combo.label}-${cIdx}`)}
            <div class="flex items-center gap-2 rounded-lg border border-emerald-800/40 bg-emerald-950/20 px-2.5 py-1.5 shadow-sm">
              <div class="flex items-center -space-x-1 shrink-0">
                {#each combo.item_ids as itemId (itemId)}
                  {@const it = knownItemMap.get(itemId)}
                  <img
                    src={it?.icon_url || getItemIconUrl(itemId)}
                    alt={it?.name || `Item #${itemId}`}
                    title={it?.name || `Item #${itemId}`}
                    class="h-6 w-6 rounded-md border border-gray-800 bg-gray-950 object-cover ring-1 ring-emerald-500/30"
                  />
                {/each}
                {#each combo.augment_ids as augId (augId)}
                  {@const aug = knownAugmentMap.get(augId) || knownAugmentMap.get(augId.toLowerCase())}
                  <AugmentIcon
                    src={aug?.icon_url ?? ""}
                    name={aug?.name ?? augId}
                    rarity={aug?.rarity ?? "silver"}
                    class="h-6 w-6 ring-1 ring-emerald-500/30"
                    roundedClass="rounded-md"
                  />
                {/each}
              </div>

              <div class="flex flex-col min-w-0">
                <span class="text-[11px] font-semibold text-gray-200 truncate max-w-[200px]" title={combo.label}>
                  {combo.label}
                </span>
                <div class="flex items-center gap-1.5 text-[9px]">
                  <span class="font-mono text-gray-400">{formatPct(combo.win_rate)} WR</span>
                  <span class="font-mono font-bold {combo.wpa >= 0 ? 'text-emerald-400' : 'text-rose-400'}">
                    {formatWpa(combo.wpa)}
                  </span>
                  <span class="text-gray-500 font-mono">({combo.sample_size}g)</span>
                </div>
              </div>
            </div>
          {/each}
        </div>
      </div>
    {/if}
  {/if}
</div>
