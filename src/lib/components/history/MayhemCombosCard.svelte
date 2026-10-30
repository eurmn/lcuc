<script lang="ts">
  import { onDestroy } from "svelte";
  import { requestStatistics } from "../../utils/statistics-data";
  import { getGlobalAramConfig, type ServerStatisticRow, type ComboComponentDto, type AgentOperationDto } from "../../api/agent";
  import AugmentIcon from "../common/AugmentIcon.svelte";
  import GameEntityPopover from "../common/GameEntityPopover.svelte";
  import { logger } from "../../utils/logger";

  export type ComboShapeFilter = "all" | "1A+1I" | "1A+2I" | "2A+1I" | "2A+2I" | "2A+3I";

  let {
    championId = 0,
    championName = "",
    currentAugmentIds = [],
    currentBuildItemIds = [],
    hideLowPickrate = false,
    agentOperations = [],
    itemsCatalog = {},
    onSelectAugment,
    onSelectItem,
  }: {
    championId?: number;
    championName?: string;
    currentAugmentIds?: string[];
    currentBuildItemIds?: number[];
    hideLowPickrate?: boolean;
    agentOperations?: AgentOperationDto[];
    itemsCatalog?: Record<number, {
      name: string;
      icon_url: string;
      item_class?: string;
      description?: string;
      price_total: number;
      categories: string[];
    }>;
    onSelectAugment?: (aug: { id: string; name: string; icon_url: string; rarity?: string; description?: string }) => void;
    onSelectItem?: (item: { id: number; name: string; icon_url: string; price_total?: number; item_class?: string; description?: string }) => void;
  } = $props();

  let allRows = $state<ServerStatisticRow[]>([]);
  let loading = $state(false);
  let shapeFilter = $state<ComboShapeFilter>("all");
  let search = $state("");
  let debouncedSearch = $state("");
  let sortKey = $state<"wpa" | "winRate" | "games">("wpa");
  let sortDescending = $state(true);
  let page = $state(1);
  let pageSize = $state(10);
  let prioritizeSynergy = $state(true);
  let searchTimer: ReturnType<typeof setTimeout> | null = null;
  let polledOperations = $state<AgentOperationDto[]>([]);
  let pollInterval: ReturnType<typeof setInterval> | null = null;

  function startPollingOperations() {
    if (pollInterval) return;
    pollInterval = setInterval(async () => {
      try {
        const cfg = await getGlobalAramConfig();
        polledOperations = cfg.operations || [];
      } catch {
        // ignore polling error
      }
    }, 350);
  }

  function stopPollingOperations() {
    if (pollInterval) {
      clearInterval(pollInterval);
      pollInterval = null;
    }
  }

  onDestroy(() => {
    stopPollingOperations();
  });

  const activeComboOp = $derived.by(() => {
    const list = polledOperations.length > 0 ? polledOperations : agentOperations;
    return (
      list.find(
        (op) =>
          op.id === "combo-mining" ||
          op.id === "global-statistics" ||
          op.id.includes("combo") ||
          op.message.toLowerCase().includes("combo") ||
          op.message.toLowerCase().includes("statistics")
      ) || null
    );
  });

  const operationTitle = $derived(
    activeComboOp?.message || "Mining combo statistics..."
  );
  const operationStep = $derived(activeComboOp?.current ?? null);
  const operationTotal = $derived(activeComboOp?.total ?? null);
  const operationPct = $derived.by(() => {
    if (operationStep !== null && operationTotal && operationTotal > 0) {
      return Math.min(100, Math.max(5, Math.round((operationStep / operationTotal) * 100)));
    }
    return null;
  });

  function formatPct(val: number | undefined): string {
    if (val === undefined || isNaN(val)) return "0.0%";
    return (val * 100).toFixed(1) + "%";
  }

  function formatWpa(val: number | undefined): string {
    if (val === undefined || isNaN(val)) return "+0.0%";
    const sign = val >= 0 ? "+" : "";
    return `${sign}${(val * 100).toFixed(1)}%`;
  }

  function formatGames(n: number | undefined): string {
    if (!n) return "0";
    if (n >= 1000) return `${(n / 1000).toFixed(1)}k`;
    return n.toString();
  }

  function isAugmentQuestItem(id: number): boolean {
    return [228002, 994403, 4403, 224403, 664403, 226668, 223069, 1111].includes(id);
  }

  function playerHasAugmentForItem(itemId: number, augIds: string[]): boolean {
    const lower = augIds.map((a) => a.toLowerCase());
    const hasAny = (keys: string[]) => lower.some((a) => keys.some((k) => a === k || a.includes(k)));
    if (itemId === 228002) return hasAny(["1156", "156", "wooglet"]);
    if ([994403, 4403, 224403, 664403].includes(itemId)) return hasAny(["1154", "154", "urf"]);
    if (itemId === 226668) return hasAny(["2141", "ultrahydra", "ultra hydra"]);
    if (itemId === 223069) return hasAny(["393", "1361", "icathia", "voidimmolation", "void immolation"]);
    if (itemId === 1111) return hasAny(["jarvan"]);
    return true;
  }

  function isLegendaryItem(comp: ComboComponentDto): boolean {
    if (comp.kind !== "item") return true;
    const id = Number(comp.id);
    if (!id || id <= 0) return false;

    // Filter out augment-exclusive reward items unless player has the augment
    if (isAugmentQuestItem(id) && !playerHasAugmentForItem(id, currentAugmentIds)) {
      return false;
    }

    const itemInfo = itemsCatalog[id];
    if (itemInfo) {
      const cls = (itemInfo.item_class || "").toLowerCase();
      if (cls === "component" || cls === "starter" || cls === "boot" || cls === "consumable") {
        return false;
      }
      if (itemInfo.price_total < 2000 && !cls.includes("legendary") && !cls.includes("mythic")) {
        return false;
      }
      return true;
    }
    // Fallback: exclude known components/starters/boots
    if ([1001, 3006, 3009, 3020, 3047, 3111, 3117, 3158, 2422].includes(id)) return false;
    if ([1054, 1055, 1056, 1082, 1083, 2055, 3184, 3850, 3851, 3853, 3854, 3855, 3857, 3858, 3859, 3860, 3862, 3863, 3864].includes(id)) return false;
    // Known basic/epic components:
    if ([1036, 1052, 1058, 1037, 1038, 1053, 3044, 3067, 1018, 3076, 3145, 1011, 1028, 4630, 3070, 2031, 2003, 220013].includes(id)) return false;
    return true;
  }

  function isBootOrStarter(comp: ComboComponentDto): boolean {
    if (comp.kind !== "item") return false;
    const name = (comp.name || "").toLowerCase();
    const itemClass = (comp.item_class || "").toLowerCase();
    if (itemClass === "boot") return true;
    const id = Number(comp.id);
    if ([1001, 3006, 3009, 3020, 3047, 3111, 3117, 3158, 2422].includes(id)) return true;
    if ([1054, 1055, 1056, 1082, 1083, 2055, 3184, 3850, 3851, 3853, 3854, 3855, 3857, 3858, 3859, 3860, 3862, 3863, 3864].includes(id)) return true;
    if (name.includes("boots") || name.includes("greaves") || name.includes("shoes") || name.includes("steelcaps") || name.includes("treads")) return true;
    return false;
  }

  function getComboShape(components: ComboComponentDto[]): "1A+1I" | "1A+2I" | "2A+1I" | "2A+2I" | "2A+3I" | null {
    const augs = components.filter((c) => c.kind === "augment").length;
    const items = components.filter((c) => c.kind === "item").length;
    if (augs === 1 && items === 1) return "1A+1I";
    if (augs === 1 && items === 2) return "1A+2I";
    if (augs === 2 && items === 1) return "2A+1I";
    if (augs === 2 && items === 2) return "2A+2I";
    if (augs === 2 && items === 3) return "2A+3I";
    return null;
  }

  function countMatches(components: ComboComponentDto[]): { augMatches: number; itemMatches: number; total: number } {
    let augMatches = 0;
    let itemMatches = 0;
    for (const comp of components) {
      if (comp.kind === "augment" && currentAugmentIds.includes(String(comp.id))) {
        augMatches++;
      } else if (comp.kind === "item" && currentBuildItemIds.includes(Number(comp.id))) {
        itemMatches++;
      }
    }
    return { augMatches, itemMatches, total: augMatches + itemMatches };
  }

  async function loadCombos() {
    loading = true;
    startPollingOperations();
    try {
      const res = await requestStatistics({
        scope: "global",
        kind: "combo",
        champion_id: championId > 0 ? championId : undefined,
        hide_low_pickrate: hideLowPickrate,
        sort_key: sortKey === "winRate" ? "winRate" : sortKey === "games" ? "games" : "wpa",
        descending: sortDescending,
        limit: 1000,
      });
      allRows = res.rows || [];
    } catch (err) {
      logger.error("frontend.mayhem_combos_card", "Failed loading combo statistics", err);
      allRows = [];
    } finally {
      loading = false;
      stopPollingOperations();
    }
  }

  $effect(() => {
    hideLowPickrate;
    championId;
    void loadCombos();
  });

  function handleSearchInput(e: Event) {
    const val = (e.target as HTMLInputElement).value;
    search = val;
    if (searchTimer) clearTimeout(searchTimer);
    searchTimer = setTimeout(() => {
      debouncedSearch = val.toLowerCase().trim();
      page = 1;
    }, 200);
  }

  function clearSearch() {
    search = "";
    debouncedSearch = "";
    page = 1;
    if (searchTimer) clearTimeout(searchTimer);
  }

  // Filter and prioritize combos
  const processedCombos = $derived.by(() => {
    if (!allRows || allRows.length === 0) return [];

    const result: Array<{
      row: ServerStatisticRow;
      shape: "1A+1I" | "1A+2I" | "2A+1I" | "2A+2I" | "2A+3I";
      matches: { augMatches: number; itemMatches: number; total: number };
    }> = [];

    for (const row of allRows) {
      if (!row.components || row.components.length === 0) continue;
      if ((row.games ?? 0) < 3) continue;

      // Filter out combos with other champions
      const champComps = row.components.filter((c) => c.kind === "champion");
      if (champComps.length > 0 && championId > 0) {
        if (!champComps.some((c) => String(c.id) === String(championId))) {
          continue;
        }
      }

      // Filter out combos containing boots, starters, or non-legendary item components
      if (row.components.some(isBootOrStarter) || !row.components.every(isLegendaryItem)) {
        continue;
      }

      // Check shape
      const shape = getComboShape(row.components);
      if (!shape) continue;

      // Filter by shape tab
      if (shapeFilter !== "all" && shape !== shapeFilter) {
        continue;
      }

      // Search filter
      if (debouncedSearch) {
        const matchesName = row.components.some((c) => c.name.toLowerCase().includes(debouncedSearch));
        if (!matchesName) continue;
      }

      const matches = countMatches(row.components);
      result.push({ row, shape, matches });
    }

    // Sort with synergy prioritization
    result.sort((a, b) => {
      if (prioritizeSynergy) {
        if (a.matches.total > 0 && b.matches.total === 0) return -1;
        if (b.matches.total > 0 && a.matches.total === 0) return 1;
        if (a.matches.total !== b.matches.total) return b.matches.total - a.matches.total;
      }

      if (sortKey === "winRate") {
        const diff = (a.row.win_rate ?? 0) - (b.row.win_rate ?? 0);
        return sortDescending ? -diff : diff;
      }
      if (sortKey === "games") {
        const diff = (a.row.games ?? 0) - (b.row.games ?? 0);
        return sortDescending ? -diff : diff;
      }
      // default: wpa
      const diff = (a.row.synergy_score ?? a.row.wpa ?? 0) - (b.row.synergy_score ?? b.row.wpa ?? 0);
      return sortDescending ? -diff : diff;
    });

    if (!debouncedSearch) {
      const MAX_PER_COMPONENT = 2;
      const counts = new Map<string, number>();
      const diversified: typeof result = [];
      for (const item of result) {
        const comps = (item.row.components ?? []).filter(
          (c) => c.kind === "item" || c.kind === "augment"
        );
        if (comps.every((c) => (counts.get(`${c.kind}:${c.id}`) ?? 0) < MAX_PER_COMPONENT)) {
          diversified.push(item);
          for (const c of comps) {
            const k = `${c.kind}:${c.id}`;
            counts.set(k, (counts.get(k) ?? 0) + 1);
          }
        }
      }
      return diversified;
    }

    return result;
  });

  const totalPages = $derived(Math.max(1, Math.ceil(processedCombos.length / pageSize)));
  const paginatedCombos = $derived(
    processedCombos.slice((page - 1) * pageSize, page * pageSize)
  );

  function handleShapeChange(shape: ComboShapeFilter) {
    shapeFilter = shape;
    page = 1;
  }

  function handleSort(key: "wpa" | "winRate" | "games") {
    if (sortKey === key) {
      sortDescending = !sortDescending;
    } else {
      sortKey = key;
      sortDescending = true;
    }
    page = 1;
  }

  function handleComponentClick(comp: ComboComponentDto) {
    if (comp.kind === "augment") {
      onSelectAugment?.({
        id: String(comp.id),
        name: comp.name,
        icon_url: comp.icon_url,
        rarity: comp.rarity,
        description: comp.description,
      });
    } else if (comp.kind === "item") {
      onSelectItem?.({
        id: Number(comp.id),
        name: comp.name,
        icon_url: comp.icon_url,
        price_total: comp.price_total,
        item_class: comp.item_class,
        description: comp.description,
      });
    }
  }
</script>

<div class="rounded-2xl border border-gray-800 bg-gray-900/60 p-4 shadow-xl space-y-4">
  <!-- Header -->
  <div class="flex flex-wrap items-center justify-between gap-2 border-b border-gray-800 pb-3">
    <div class="flex items-center gap-2">
      <span class="icon-[heroicons--sparkles] h-5 w-5 text-amber-400"></span>
      <div>
        <h2 class="text-sm font-bold text-gray-100 flex items-center gap-2">
          ARAM Mayhem Combos
          {#if championName}
            <span class="text-xs font-normal text-gray-400">· {championName}</span>
          {/if}
          <span class="rounded bg-amber-950/80 border border-amber-700/50 px-1.5 py-0.2 text-[10px] font-semibold text-amber-300">
            Augment + Item Builds
          </span>
        </h2>
        <p class="text-[11px] text-gray-400">
          Click any augment or item to add to your build
        </p>
      </div>
    </div>

    <!-- Synergy Toggle & Count -->
    <div class="flex items-center gap-2">
      {#if loading}
        <div class="inline-flex items-center gap-1.5 rounded-lg border border-amber-500/40 bg-amber-950/40 px-2 py-1 text-[11px] font-medium text-amber-300">
          <span class="icon-[heroicons--arrow-path] h-3 w-3 animate-spin text-amber-400"></span>
          <span>{operationTitle}</span>
          {#if operationPct !== null}
            <span class="font-mono text-[10px] text-amber-200">({operationPct}%)</span>
          {:else if operationStep !== null && operationTotal}
            <span class="font-mono text-[10px] text-amber-200">({operationStep}/{operationTotal})</span>
          {/if}
        </div>
      {/if}
      <button
        type="button"
        onclick={() => (prioritizeSynergy = !prioritizeSynergy)}
        class="inline-flex items-center gap-1 rounded-lg px-2 py-1 text-[11px] font-medium transition-colors border {prioritizeSynergy
          ? 'bg-purple-950/60 border-purple-600/70 text-purple-300 shadow-sm'
          : 'bg-gray-800/60 border-gray-700 text-gray-400 hover:text-gray-200'}"
        title="Prioritize combos containing your currently chosen augments or build items"
      >
        <span class="icon-[heroicons--bolt] h-3.5 w-3.5 {prioritizeSynergy ? 'text-purple-400' : 'text-gray-500'}"></span>
        Synergy First
      </button>
      <span class="text-[11px] font-mono text-gray-500">
        {processedCombos.length} combos
      </span>
    </div>
  </div>

  <!-- Combo Shape Tabs (1A+1I, 1A+2I, 2A+1I, 2A+2I, 2A+3I) -->
  <div class="flex items-center gap-1.5 overflow-x-auto pb-1 scrollbar-thin">
    {#each [
      { id: "all", label: "All Shapes" },
      { id: "1A+1I", label: "1A + 1I" },
      { id: "1A+2I", label: "1A + 2I" },
      { id: "2A+1I", label: "2A + 1I" },
      { id: "2A+2I", label: "2A + 2I" },
      { id: "2A+3I", label: "2A + 3I" }
    ] as tab}
      <button
        type="button"
        onclick={() => handleShapeChange(tab.id as ComboShapeFilter)}
        class="whitespace-nowrap px-2.5 py-1 text-xs font-semibold rounded-lg transition-colors border {shapeFilter === tab.id
          ? 'bg-amber-500/20 text-amber-300 border-amber-500/60 shadow-sm'
          : 'bg-gray-800/40 text-gray-400 border-transparent hover:bg-gray-800/80 hover:text-gray-200'}"
      >
        {tab.label}
      </button>
    {/each}
  </div>

  <!-- Search and Sort Toolbar -->
  <div class="flex flex-wrap items-center justify-between gap-2 text-xs">
    <!-- Search Bar -->
    <div class="relative flex-1 min-w-[160px]">
      <span class="icon-[heroicons--magnifying-glass] absolute left-2.5 top-1/2 -translate-y-1/2 h-3.5 w-3.5 text-gray-400"></span>
      <input
        type="text"
        placeholder="Filter by item or augment..."
        value={search}
        oninput={handleSearchInput}
        class="w-full rounded-lg border border-gray-700 bg-gray-950/80 pl-8 pr-7 py-1 text-xs text-gray-200 placeholder-gray-500 focus:border-amber-500 focus:outline-none"
      />
      {#if search}
        <button
          type="button"
          aria-label="Clear search"
          title="Clear search"
          onclick={clearSearch}
          class="absolute right-2 top-1/2 -translate-y-1/2 text-gray-400 hover:text-gray-200"
        >
          <span class="icon-[heroicons--x-mark] h-3.5 w-3.5"></span>
        </button>
      {/if}
    </div>

    <!-- Sort Controls -->
    <div class="flex items-center gap-1">
      <button
        type="button"
        onclick={() => handleSort("wpa")}
        class="flex items-center gap-1 rounded px-2 py-1 text-[11px] font-semibold border transition-colors {sortKey === 'wpa'
          ? 'bg-emerald-950/70 border-emerald-700 text-emerald-300'
          : 'bg-gray-800/40 border-gray-700 text-gray-400 hover:text-gray-200'}"
      >
        WPA
        {#if sortKey === "wpa"}
          <span class="icon-[heroicons--chevron-{sortDescending ? 'down' : 'up'}] h-3 w-3"></span>
        {/if}
      </button>

      <button
        type="button"
        onclick={() => handleSort("winRate")}
        class="flex items-center gap-1 rounded px-2 py-1 text-[11px] font-semibold border transition-colors {sortKey === 'winRate'
          ? 'bg-purple-950/70 border-purple-700 text-purple-300'
          : 'bg-gray-800/40 border-gray-700 text-gray-400 hover:text-gray-200'}"
      >
        Win%
        {#if sortKey === "winRate"}
          <span class="icon-[heroicons--chevron-{sortDescending ? 'down' : 'up'}] h-3 w-3"></span>
        {/if}
      </button>

      <button
        type="button"
        onclick={() => handleSort("games")}
        class="flex items-center gap-1 rounded px-2 py-1 text-[11px] font-semibold border transition-colors {sortKey === 'games'
          ? 'bg-gray-700/60 border-gray-600 text-gray-200'
          : 'bg-gray-800/40 border-gray-700 text-gray-400 hover:text-gray-200'}"
      >
        Games
        {#if sortKey === "games"}
          <span class="icon-[heroicons--chevron-{sortDescending ? 'down' : 'up'}] h-3 w-3"></span>
        {/if}
      </button>
    </div>
  </div>

  <!-- Combos List / Table -->
  {#if loading && allRows.length === 0}
    <div class="flex flex-col items-center justify-center gap-3.5 rounded-xl border border-gray-800 bg-gray-950/50 p-6 text-xs text-gray-300">
      <div class="flex items-center gap-2.5">
        <span class="icon-[heroicons--arrow-path] h-4 w-4 animate-spin text-amber-400"></span>
        <span class="font-medium text-gray-200">{operationTitle}</span>
        {#if operationStep !== null && operationTotal}
          <span class="rounded bg-amber-950/80 border border-amber-700/60 px-2 py-0.5 text-[10px] font-semibold text-amber-300">
            Step {operationStep} of {operationTotal}
          </span>
        {/if}
      </div>

      <!-- Progress bar track -->
      <div class="w-full max-w-md h-2 rounded-full bg-gray-800 overflow-hidden border border-gray-700/60">
        <div
          class="h-full bg-gradient-to-r from-cyan-500 via-amber-400 to-purple-500 transition-all duration-300 ease-out shadow-sm"
          style="width: {operationPct ?? 35}%;"
        ></div>
      </div>

      <!-- Stage breakdown pills -->
      <div class="flex flex-wrap items-center justify-center gap-1.5 pt-1 text-[10px]">
        {#each [
          { step: 1, label: "1. Load Facts" },
          { step: 2, label: "2. Frequency" },
          { step: 3, label: "3. Synergy Pairs" },
          { step: 4, label: "4. Win Rate & WPA" },
        ] as stage}
          <span
            class="rounded-md px-2 py-0.5 font-medium border transition-colors {operationStep === stage.step
              ? 'bg-amber-500/20 text-amber-300 border-amber-500/60 shadow-sm'
              : operationStep !== null && operationStep > stage.step
                ? 'bg-emerald-950/40 text-emerald-400 border-emerald-700/40'
                : 'bg-gray-900/60 text-gray-500 border-gray-800'}"
          >
            {stage.label}
          </span>
        {/each}
      </div>
    </div>
  {:else if paginatedCombos.length === 0}
    <div class="flex h-36 flex-col items-center justify-center rounded-xl border border-gray-800 bg-gray-950/40 text-xs text-gray-500 p-4 text-center">
      <span class="icon-[heroicons--cube-transparent] h-8 w-8 text-gray-600 mb-1"></span>
      <span>No combos match current filters</span>
      {#if search || shapeFilter !== "all"}
        <button
          type="button"
          onclick={() => {
            clearSearch();
            shapeFilter = "all";
          }}
          class="mt-2 text-amber-400 hover:underline text-[11px]"
        >
          Reset filters
        </button>
      {/if}
    </div>
  {:else}
    <div class="space-y-2">
      {#each paginatedCombos as { row, matches }, idx (row.id || idx)}
        {@const hasSynergy = matches.total > 0}
        <div
          class="flex items-center justify-between gap-3 p-2.5 rounded-xl border transition-all {hasSynergy
            ? 'border-purple-600/50 bg-purple-950/20 hover:border-purple-500 shadow-sm'
            : 'border-gray-800 bg-gray-950/60 hover:border-gray-700'}"
        >
          <!-- Left: Components Row with '+' separators -->
          <div class="flex items-center gap-1.5 flex-wrap min-w-0">
            {#if hasSynergy}
              <span class="rounded bg-purple-900/60 border border-purple-500/60 px-1 py-0.2 text-[9px] font-bold text-purple-200 shrink-0">
                Synergy
              </span>
            {/if}

            {#if row.components}
              {#each row.components as comp, cIdx}
                {@const compId = `combo-comp-${page}-${idx}-${cIdx}`}
                {@const isAugment = comp.kind === "augment"}
                {@const isItem = comp.kind === "item"}
                {@const isChamp = comp.kind === "champion"}
                {@const isAugMatch = isAugment && currentAugmentIds.includes(String(comp.id))}
                {@const isItemMatch = isItem && currentBuildItemIds.includes(Number(comp.id))}
                {@const isCompMatched = isAugMatch || isItemMatch}

                {#if cIdx > 0}
                  <span class="text-gray-500 text-[10px] font-bold select-none">+</span>
                {/if}

                <div class="relative shrink-0 group">
                  <button
                    type="button"
                    id={compId}
                    onclick={() => handleComponentClick(comp)}
                    class="relative block rounded-lg transition-transform hover:scale-110 active:scale-95 focus:outline-none cursor-pointer {isCompMatched
                      ? 'ring-2 ring-emerald-400 ring-offset-1 ring-offset-gray-950'
                      : ''}"
                    title={`Click to add ${comp.name} to ${isAugment ? 'augments' : 'build'}`}
                  >
                    {#if isAugment}
                      <AugmentIcon
                        src={comp.icon_url}
                        name={comp.name}
                        rarity={comp.rarity}
                        class="h-8 w-8"
                        roundedClass="rounded-lg"
                      />
                    {:else if isItem}
                      {#if comp.icon_url}
                        <img
                          src={comp.icon_url}
                          alt={comp.name}
                          class="h-8 w-8 rounded-lg object-cover border border-gray-700 shadow-sm bg-gray-950"
                        />
                      {:else}
                        <div class="h-8 w-8 rounded-lg bg-gray-800 border border-gray-700 flex items-center justify-center text-[10px] font-bold text-gray-300">
                          {comp.name.slice(0, 1)}
                        </div>
                      {/if}
                    {:else if isChamp}
                      {#if comp.icon_url}
                        <img
                          src={comp.icon_url}
                          alt={comp.name}
                          class="h-8 w-8 rounded-full object-cover border border-cyan-600/70 shadow-sm"
                        />
                      {:else}
                        <div class="h-8 w-8 rounded-full bg-cyan-950 border border-cyan-700 flex items-center justify-center text-[10px] font-bold text-cyan-300">
                          {comp.name.slice(0, 1)}
                        </div>
                      {/if}
                    {/if}

                    <!-- Matched Indicator Dot -->
                    {#if isCompMatched}
                      <span class="absolute -top-1 -right-1 h-2.5 w-2.5 rounded-full bg-emerald-400 border border-gray-950 shadow-sm" title="Already in your build/augments"></span>
                    {/if}
                  </button>

                  <GameEntityPopover
                    targetId={compId}
                    name={comp.name}
                    iconUrl={comp.icon_url}
                    description={comp.description}
                    rarity={comp.rarity}
                    itemClass={comp.item_class}
                    priceTotal={comp.price_total}
                    categories={comp.categories}
                    placement="top"
                  />
                </div>
              {/each}
            {/if}
          </div>

          <!-- Right: Metrics (WPA, Win Rate, Games) -->
          <div class="flex items-center gap-2.5 shrink-0 text-right">
            <div>
              <div class="text-xs font-mono font-bold {(row.wpa ?? 0) >= 0 ? 'text-emerald-400' : 'text-rose-400'}">
                {formatWpa(row.wpa)}
              </div>
              <div class="text-[10px] font-mono text-purple-300">
                {formatPct(row.win_rate)}
              </div>
            </div>
            <div class="text-[10px] font-mono text-gray-500 w-10">
              {formatGames(row.games)}g
            </div>
          </div>
        </div>
      {/each}
    </div>

    <!-- Pagination Controls -->
    {#if totalPages > 1}
      <div class="flex items-center justify-between border-t border-gray-800 pt-3 text-xs text-gray-400">
        <span>Page {page} of {totalPages}</span>
        <div class="flex items-center gap-1.5">
          <button
            type="button"
            disabled={page <= 1}
            onclick={() => (page = Math.max(1, page - 1))}
            class="rounded px-2 py-1 border border-gray-700 bg-gray-800/40 text-gray-300 disabled:opacity-40 disabled:cursor-not-allowed hover:bg-gray-800"
          >
            Prev
          </button>
          <button
            type="button"
            disabled={page >= totalPages}
            onclick={() => (page = Math.min(totalPages, page + 1))}
            class="rounded px-2 py-1 border border-gray-700 bg-gray-800/40 text-gray-300 disabled:opacity-40 disabled:cursor-not-allowed hover:bg-gray-800"
          >
            Next
          </button>
        </div>
      </div>
    {/if}
  {/if}
</div>
