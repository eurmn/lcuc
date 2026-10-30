<script lang="ts">
  import type { MayhemBuildItem } from "../../types/champ_select";
  import GameEntityPopover from "../common/GameEntityPopover.svelte";

  let {
    buildSlots = Array(6).fill(null),
    selectedSlotIndex = null,
    starterItem = null,
    onSelectSlot,
    onClearSlot,
    onItemClick,
    onReset,
  }: {
    buildSlots: (MayhemBuildItem | null)[];
    selectedSlotIndex: number | null;
    starterItem?: MayhemBuildItem | null;
    onSelectSlot: (index: number) => void;
    onClearSlot: (index: number) => void;
    onItemClick?: (item: MayhemBuildItem, index: number) => void;
    onReset?: () => void;
  } = $props();

  const totalGold = $derived(
    buildSlots.reduce((sum, item) => sum + (item?.price_total ?? 0), starterItem?.price_total ?? 0)
  );

  const equippedCount = $derived(
    buildSlots.filter((s): s is MayhemBuildItem => s !== null).length
  );

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

  function handleSlotPress(idx: number) {
    onSelectSlot(idx);
    const item = buildSlots[idx];
    if (item && onItemClick) {
      onItemClick(item, idx);
    }
  }
</script>

<div class="rounded-xl border border-gray-800 bg-gray-900/80 p-4 shadow-xl backdrop-blur-sm space-y-3">
  <!-- Header: Equipped count, Gold sum, Reset button -->
  <div class="flex items-center justify-between border-b border-gray-800 pb-2.5">
    <div class="flex items-center gap-2">
      <div class="flex h-7 w-7 items-center justify-center rounded-lg bg-amber-500/10 text-amber-400 border border-amber-500/30">
        <span class="icon-[heroicons--shopping-bag] text-sm"></span>
      </div>
      <div>
        <h3 class="text-xs font-bold uppercase tracking-wider text-gray-200">
          Simulated Build Inventory
        </h3>
        <p class="text-[10px] text-gray-400">
          Target slot to equip recommendations or click to remove
        </p>
      </div>
    </div>

    <div class="flex items-center gap-2.5">
      <span class="text-xs font-mono font-bold text-amber-400 flex items-center gap-1">
        <span class="icon-[heroicons--circle-stack] text-xs"></span>
        {totalGold.toLocaleString()}g
      </span>
      <span class="rounded bg-gray-800/80 px-2 py-0.5 text-[10px] font-mono font-medium text-gray-300 border border-gray-700">
        {equippedCount}/6 Slots
      </span>
      {#if onReset && equippedCount > 0}
        <button
          type="button"
          onclick={onReset}
          class="flex items-center gap-1 rounded bg-rose-950/40 hover:bg-rose-900/50 border border-rose-800/50 px-2 py-0.5 text-[10px] font-medium text-rose-300 transition-colors"
          title="Reset all build slots"
        >
          <span class="icon-[heroicons--arrow-path] text-xs"></span>
          Reset
        </button>
      {/if}
    </div>
  </div>

  <!-- Build Slots Layout: Optional Starter + 6 Item Inventory Slots -->
  <div class="flex flex-col sm:flex-row items-center gap-3">
    <!-- Optional Starter Item -->
    {#if starterItem}
      <div class="flex flex-col items-center justify-center p-2 rounded-lg border border-gray-800 bg-gray-950/40 w-full sm:w-24 shrink-0">
        <span class="text-[9px] uppercase font-bold text-gray-400 mb-1">Starter</span>
        <div id="sim-starter-item" class="relative h-11 w-11 rounded-lg overflow-hidden border border-gray-700 bg-gray-950">
          <img src={starterItem.icon_url} alt={starterItem.name} class="h-full w-full object-cover" />
        </div>
        <span class="text-[10px] font-medium text-gray-200 truncate max-w-full text-center mt-1">
          {starterItem.name}
        </span>
        <span class="text-[9px] font-mono text-amber-400/90">{starterItem.price_total}g</span>
        <GameEntityPopover
          targetId="sim-starter-item"
          name={starterItem.name}
          iconUrl={starterItem.icon_url}
          description={starterItem.description}
          priceTotal={starterItem.price_total}
          itemClass={starterItem.item_class}
          placement="top"
        />
      </div>
      <div class="hidden sm:block w-px bg-gray-800 self-stretch my-0.5"></div>
    {/if}

    <!-- 6 Active Build Slots Grid -->
    <div class="flex-1 grid grid-cols-2 sm:grid-cols-3 lg:grid-cols-6 gap-2 w-full">
      {#each buildSlots as item, idx (`sim-slot-${idx}`)}
        {#if item}
          <div
            id={`sim-item-${item.id}-${idx}`}
            role="button"
            tabindex="0"
            onclick={() => handleSlotPress(idx)}
            onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') handleSlotPress(idx); }}
            class="group relative flex flex-col items-center justify-center p-2 rounded-lg border transition-all cursor-pointer {selectedSlotIndex === idx
              ? 'border-amber-400 bg-amber-950/30 ring-2 ring-amber-400/40 shadow-lg'
              : 'border-gray-800 bg-gray-950/50 hover:border-gray-700'}"
            title="Click to select or replace slot"
          >
            <!-- Clear button on hover -->
            <button
              type="button"
              onclick={(e) => { e.stopPropagation(); onClearSlot(idx); }}
              class="absolute -top-1.5 -right-1.5 hidden group-hover:flex h-4 w-4 items-center justify-center rounded-full bg-rose-600 hover:bg-rose-500 text-white text-[10px] shadow z-10 font-bold"
              title="Clear item from slot"
            >
              ×
            </button>

            <div class="relative h-11 w-11 rounded-lg overflow-hidden border border-gray-700 bg-gray-950 mb-1">
              <img src={item.icon_url} alt={item.name} class="h-full w-full object-cover" />
              <span class="absolute left-0 top-0 rounded-br bg-gray-950/80 px-1 text-[9px] font-bold text-gray-300">
                {idx + 1}
              </span>
              {#if selectedSlotIndex === idx}
                <span class="absolute inset-0 bg-amber-500/20 ring-1 ring-inset ring-amber-400 pointer-events-none"></span>
              {/if}
            </div>

            <span class="text-[10px] font-medium text-gray-200 truncate max-w-full text-center">
              {item.name}
            </span>

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

            <GameEntityPopover
              targetId={`sim-item-${item.id}-${idx}`}
              name={item.name}
              iconUrl={item.icon_url}
              description={item.description}
              priceTotal={item.price_total}
              itemClass={item.item_class}
              placement="top"
            />
          </div>
        {:else}
          <!-- Empty Slot Placeholder -->
          <button
            type="button"
            onclick={() => handleSlotPress(idx)}
            class="flex flex-col items-center justify-center p-2 rounded-lg border-2 border-dashed transition-all cursor-pointer {selectedSlotIndex === idx
              ? 'border-amber-400 bg-amber-950/30 text-amber-300 ring-2 ring-amber-400/40'
              : 'border-gray-800/90 bg-gray-950/30 text-gray-600 hover:border-gray-600 hover:text-gray-400'}"
            title="Click to target this slot for next item pick"
          >
            <div class="h-11 w-11 rounded-lg border border-dashed {selectedSlotIndex === idx ? 'border-amber-400/80 bg-amber-500/10' : 'border-gray-800 bg-gray-900/40'} flex items-center justify-center mb-1">
              <span class="icon-[heroicons--plus] h-5 w-5 {selectedSlotIndex === idx ? 'text-amber-400 animate-pulse' : 'text-gray-500'}"></span>
            </div>
            <span class="text-[10px] font-medium {selectedSlotIndex === idx ? 'text-amber-300 font-bold' : 'text-gray-500'}">
              Slot {idx + 1}
            </span>
            <span class="text-[9px] {selectedSlotIndex === idx ? 'text-amber-400 font-semibold' : 'text-gray-600'}">
              {selectedSlotIndex === idx ? "Targeted" : "Empty"}
            </span>
          </button>
        {/if}
      {/each}
    </div>
  </div>
</div>
