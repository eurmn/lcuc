<script lang="ts">
  import type { StatShardOption, StatShardsStats } from "../../types/rune_builder";
  import {
    formatGameCount,
    formatOccurrence,
    formatPickRate,
    getPickRateOpacity,
    getWpaBarMetrics,
    getWpaText,
  } from "../../utils/wpa";

  let {
    statShards,
    selectedShardIds = [],
    isEditing = true,
    onSelectShard,
  }: {
    statShards: StatShardsStats;
    selectedShardIds: number[];
    isEditing?: boolean;
    onSelectShard: (shardId: number, rowIndex: number) => void;
  } = $props();

  // Official League client stat shard order per row:
  // Row 1 (Offense): Adaptive Force (5008), Attack Speed (5005), Ability Haste / CDR (5007)
  // Row 2 (Flex): Adaptive Force (5008), Movement Speed (5010), Scaling Health (5001)
  // Row 3 (Defense): Flat Health (5011), Tenacity / Slow Resist (5013), Scaling Health (5001)
  const SHARD_ROW_ORDERS: number[][] = [
    [5008, 5005, 5007],
    [5008, 5010, 5001],
    [5011, 5013, 5001],
  ];

  function getSortedOptions(options: StatShardOption[] | undefined, rowIndex: number): StatShardOption[] {
    if (!options) return [];
    const order = SHARD_ROW_ORDERS[rowIndex] || [];
    return [...options].sort((a, b) => {
      const posA = order.indexOf(a.id);
      const posB = order.indexOf(b.id);
      return (posA !== -1 ? posA : 999) - (posB !== -1 ? posB : 999);
    });
  }

  const rows = $derived([
    { label: "Offense", options: getSortedOptions(statShards?.offense, 0) },
    { label: "Flex", options: getSortedOptions(statShards?.flex, 1) },
    { label: "Defense", options: getSortedOptions(statShards?.defense, 2) },
  ]);
</script>

<div class="flex flex-col gap-3 rounded-2xl border border-gray-800 bg-gray-950/80 p-4 shadow-sm">
  <div class="flex items-center justify-between pb-1">
    <h4 class="text-xs font-bold uppercase tracking-wider text-gray-300">Stat Shards</h4>
  </div>

  <div class="flex flex-col gap-2.5">
    {#each rows as row, rowIndex (rowIndex)}
      <div class="flex flex-col gap-1">
        <span class="text-[10px] font-bold uppercase tracking-wider text-gray-500 px-0.5">
          {row.label}
        </span>

        <div class="grid grid-cols-3 gap-2">
          {#each row.options as option (option.id)}
            {@const isSelected = selectedShardIds[rowIndex] === option.id}
            {@const hasData = option.occurrence > 0 || option.pick_rate_pct > 0}
            {@const opacityVal = getPickRateOpacity(option.pick_rate_pct)}
            {@const wpaMetrics = getWpaBarMetrics(option.wpa, hasData)}
            {@const wpaText = getWpaText(option.wpa, hasData)}
            {@const gameCountText = formatGameCount(option.occurrence, hasData)}

            <button
              type="button"
              disabled={!isEditing}
              title={hasData
                ? `${option.name} — ${formatPickRate(option.pick_rate_pct)} (${formatOccurrence(option.occurrence)} picks), ${wpaText}`
                : `${option.name} — No statistical information available`}
              onclick={() => isEditing && onSelectShard(option.id, rowIndex)}
              class={isSelected
                ? (isEditing
                  ? "flex items-center gap-2.5 rounded-xl border-2 border-purple-500 bg-gray-900 p-2 text-left shadow-sm cursor-pointer transition-all"
                  : "flex items-center gap-2.5 rounded-xl border-2 border-purple-500/80 bg-gray-900 p-2 text-left shadow-sm select-none cursor-default")
                : (isEditing
                  ? "flex items-center gap-2.5 rounded-xl border-2 border-gray-700 bg-gray-900/80 p-2 text-left opacity-60 hover:opacity-100 hover:bg-gray-800 cursor-pointer transition-all"
                  : "flex items-center gap-2.5 rounded-xl border-2 border-gray-800/60 bg-gray-900/40 p-2 text-left opacity-35 select-none cursor-default")}
            >
              <!-- Shard Icon -->
              <div class="flex h-8 w-8 shrink-0 items-center justify-center rounded-lg border border-gray-700/60 bg-gray-950 p-1">
                {#if option.icon_url}
                  <img src={option.icon_url} alt={option.name} class="h-full w-full object-contain" />
                {:else}
                  <div class="h-full w-full bg-gray-800"></div>
                {/if}
              </div>

              <!-- Telemetry Info: Games Count + White Bar, Divider, WPA Text + Bi-Directional Bar -->
              <div class="flex flex-col min-w-0 flex-1">
                {#if hasData}
                  <!-- 1. Games Count & White Pick Rate Bar -->
                  <div class="flex flex-col gap-1">
                    <div class="flex items-center justify-between">
                      <span class="text-[11px] font-normal text-white leading-none" style="opacity: {opacityVal}">
                        {gameCountText}
                      </span>
                    </div>
                    <div class="h-1 w-full rounded-full bg-gray-950 overflow-hidden shadow-inner">
                      <div
                        class="h-full rounded-full bg-white transition-all"
                        style="width: {Math.min(100, Math.max(4, option.pick_rate_pct))}%; opacity: {opacityVal}"
                      ></div>
                    </div>
                  </div>

                  <!-- Divider between Pick Rate and WPA -->
                  <div class="h-[1px] w-full bg-gray-800/80 my-1"></div>

                  <!-- 2. WPA Text & Bi-Directional Center-Origin Bar -->
                  <div class="flex flex-col gap-1">
                    <div class="flex items-center justify-between">
                      <span class="text-[10px] font-normal {wpaMetrics.textColorClass} leading-none">
                        {wpaText}
                      </span>
                    </div>
                    <div class="relative h-1 w-full rounded-full bg-gray-950 overflow-hidden shadow-inner">
                      <!-- Center divider (50% baseline) -->
                      <div class="absolute left-1/2 top-0 bottom-0 w-[1px] -translate-x-1/2 bg-gray-700 z-10"></div>
                      {#if option.wpa > 0}
                        <div
                          class="absolute left-1/2 top-0 bottom-0 bg-emerald-400 rounded-r-full"
                          style="width: {wpaMetrics.barWidthPct}%"
                        ></div>
                      {:else if option.wpa < 0}
                        <div
                          class="absolute right-1/2 top-0 bottom-0 bg-red-500 rounded-l-full"
                          style="width: {wpaMetrics.barWidthPct}%"
                        ></div>
                      {/if}
                    </div>
                  </div>
                {:else}
                  <div class="flex flex-col gap-1 py-1">
                    <span class="text-[10px] font-normal text-gray-400">No Data</span>
                    <div class="h-1 w-full rounded-full bg-gray-950"></div>
                  </div>
                {/if}
              </div>
            </button>
          {/each}
        </div>
      </div>
    {/each}
  </div>
</div>
