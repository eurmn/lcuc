<script lang="ts">
  import { getMayhemAugments } from "../../api/champ_select";
  import type { MayhemAugmentItem } from "../../types/champ_select";
  import AugmentIcon from "../common/AugmentIcon.svelte";
  import { logger } from "../../utils/logger";

  // Slot i holds the augment picked at levels 3, 7, 11, 15 (null = empty).
  let {
    championId,
    slots = $bindable<(MayhemAugmentItem | null)[]>([null, null, null, null]),
    fallbackUsed = false,
    onChange,
  }: {
    championId: number;
    slots?: (MayhemAugmentItem | null)[];
    fallbackUsed?: boolean;
    onChange?: (ids: string[]) => void;
  } = $props();

  const LEVELS = [3, 7, 11, 15];

  let openSlot = $state<number | null>(null);
  let search = $state("");
  let results = $state<MayhemAugmentItem[]>([]);
  let loading = $state(false);

  const pickedIds = $derived(slots.filter((s): s is MayhemAugmentItem => !!s).map((s) => s.id));

  function emit() {
    onChange?.(pickedIds);
  }

  async function loadResults() {
    loading = true;
    try {
      const res = await getMayhemAugments({
        champion_id: championId,
        search,
        page: 1,
        page_size: 24,
        sort_key: "wpa",
        descending: true,
        with_augment_ids: pickedIds,
      });
      results = res.items;
    } catch (err) {
      logger.error("frontend.mayhem_augment_picker", "Failed loading augments", err);
      results = [];
    } finally {
      loading = false;
    }
  }

  // Reload on open and on search change (backend already excludes picked augments).
  $effect(() => {
    if (openSlot === null) return;
    search;
    void loadResults();
  });

  function pick(aug: MayhemAugmentItem) {
    if (openSlot === null) return;
    slots = slots.map((s, i) => (i === openSlot ? aug : s));
    openSlot = null;
    search = "";
    emit();
  }

  function clear(i: number) {
    slots = slots.map((s, idx) => (idx === i ? null : s));
    emit();
  }

  function clickSlot(i: number) {
    if (slots[i]) clear(i);
    else openSlot = i;
  }
</script>

<div class="rounded-2xl border border-gray-800 bg-gray-900/60 p-4 shadow-xl space-y-3">
  <div class="flex items-center justify-between">
    <h2 class="text-xs font-semibold uppercase tracking-wider text-gray-400 flex items-center gap-1.5">
      <span class="icon-[heroicons--sparkles] h-4 w-4 text-purple-400"></span>
      In-Game Augments
    </h2>
    {#if fallbackUsed}
      <span class="rounded border border-amber-800/60 bg-amber-950 px-2 py-0.5 text-[10px] text-amber-300">
        Few games with this combo — build uses a broader sample
      </span>
    {/if}
  </div>

  <div class="grid grid-cols-4 gap-3">
    {#each slots as slot, i (i)}
      <button
        type="button"
        onclick={() => clickSlot(i)}
        title={slot ? "Click to clear" : "Click to pick augment"}
        class="flex h-40 cursor-pointer flex-col items-center justify-center gap-2 rounded-xl border p-2 transition-colors {slot
          ? 'border-purple-700/70 bg-purple-950/30 hover:border-rose-600/70'
          : 'border-dashed border-gray-700 bg-gray-950/40 hover:border-purple-500'}"
      >
        <span class="text-[10px] font-semibold uppercase text-gray-500">Level {LEVELS[i]}</span>
        {#if slot}
          <AugmentIcon src={slot.icon_url} name={slot.name} rarity={slot.rarity} class="h-14 w-14" roundedClass="rounded-lg" />
          <span class="text-[11px] font-semibold text-gray-200 text-center leading-tight line-clamp-2">{slot.name}</span>
        {:else}
          <span class="icon-[heroicons--plus] h-8 w-8 text-gray-600"></span>
        {/if}
      </button>
    {/each}
  </div>

  {#if openSlot !== null}
    <div class="rounded-xl border border-gray-700 bg-gray-950/90 p-3 space-y-2">
      <div class="flex items-center gap-2">
        <input
          type="text"
          placeholder="Search augments..."
          bind:value={search}
          class="flex-1 rounded-lg border border-gray-700 bg-gray-900 px-2.5 py-1 text-xs text-gray-200 placeholder-gray-500 focus:border-purple-500 focus:outline-none"
        />
        <button
          type="button"
          title="Close"
          onclick={() => (openSlot = null)}
          class="cursor-pointer text-gray-400 hover:text-gray-200"
        >
          <span class="icon-[heroicons--x-mark] h-4 w-4"></span>
        </button>
      </div>
      <div class="grid max-h-64 grid-cols-2 gap-1.5 overflow-y-auto pr-1">
        {#if loading && results.length === 0}
          <span class="col-span-2 py-4 text-center text-xs text-gray-500">Loading...</span>
        {/if}
        {#each results as aug (aug.id)}
          <button
            type="button"
            onclick={() => pick(aug)}
            class="flex cursor-pointer items-center gap-2 rounded-lg border border-gray-800 bg-gray-900/60 p-1.5 text-left hover:border-purple-500"
          >
            <AugmentIcon src={aug.icon_url} name={aug.name} rarity={aug.rarity} class="h-8 w-8" />
            <div class="flex min-w-0 flex-col">
              <span class="truncate text-xs font-semibold text-gray-200">{aug.name}</span>
              <span class="text-[10px] font-mono {aug.wpa >= 0 ? 'text-emerald-400' : 'text-rose-400'}">
                {aug.games > 0 ? `${aug.wpa >= 0 ? "+" : ""}${(aug.wpa * 100).toFixed(1)}% · ${aug.games}g` : "no data"}
              </span>
            </div>
          </button>
        {/each}
      </div>
    </div>
  {/if}
</div>
