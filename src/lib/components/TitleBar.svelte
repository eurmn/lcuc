<script lang="ts">
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import NavTabs from "./navigation/NavTabs.svelte";
  import LogsModal from "./common/LogsModal.svelte";
  import type { AgentOperation, GlobalAramPatchBreakdown } from "../api/agent";

  interface Props {
    activeTab?: "builds" | "mayhem" | "profile";
    onTabChange?: (tab: "builds" | "mayhem" | "profile") => void;
    agentOperations?: AgentOperation[];
    globalHasPatchMismatch?: boolean;
    globalCollectorState?: string;
    globalGames?: number;
    globalLimit?: number;
    globalCurrentPatch?: string | null;
    globalPatches?: GlobalAramPatchBreakdown[];
    globalOutdatedGames?: number;
    globalQueuedJobs?: number;
    crawlerPaused?: boolean;
    crawlerRateText?: string;
    crawlerEtaText?: string;
    hideLowPickrate?: boolean;
    onToggleHideLowPickrate?: () => void;
    isLcuConnected?: boolean;
    onSaveLimit?: (limit: number) => Promise<void>;
    onResetGlobalDatabase?: () => Promise<void>;
    onPauseCrawler?: (paused: boolean) => Promise<void>;
    onStopCrawler?: () => Promise<void>;
  }

  let {
    activeTab = "builds",
    onTabChange,
    agentOperations = [],
    globalHasPatchMismatch = false,
    globalCollectorState = "unknown",
    globalGames = 0,
    globalLimit = 1000,
    globalCurrentPatch = null,
    globalPatches = [],
    globalOutdatedGames = 0,
    globalQueuedJobs = 0,
    crawlerPaused = false,
    crawlerRateText = "0.0 match/min",
    crawlerEtaText = "",
    hideLowPickrate = false,
    onToggleHideLowPickrate,
    isLcuConnected = false,
    onSaveLimit,
    onResetGlobalDatabase,
    onPauseCrawler,
    onStopCrawler,
  }: Props = $props();

  const appWindow = getCurrentWindow();

  let logsModalOpen = $state(false);
  let settingsOpen = $state(false);
  let localLimit = $state(1000);
  let confirmingReset = $state(false);
  let savingLimit = $state(false);
  let pausingCrawler = $state(false);
  let stoppingCrawler = $state(false);
  let resettingDb = $state(false);

  $effect(() => {
    if (!settingsOpen) {
      localLimit = globalLimit;
      confirmingReset = false;
    }
  });

  function minimize() {
    appWindow.minimize();
  }

  function toggleMaximize() {
    appWindow.toggleMaximize();
  }

  function close() {
    appWindow.close();
  }

  async function handleSaveLimit() {
    if (!onSaveLimit) return;
    savingLimit = true;
    try {
      await onSaveLimit(localLimit);
    } finally {
      savingLimit = false;
    }
  }

  async function handleTogglePause() {
    if (!onPauseCrawler) return;
    pausingCrawler = true;
    try {
      await onPauseCrawler(!crawlerPaused);
    } finally {
      pausingCrawler = false;
    }
  }

  async function handleStopCrawler() {
    if (!onStopCrawler) return;
    stoppingCrawler = true;
    try {
      await onStopCrawler();
    } finally {
      stoppingCrawler = false;
    }
  }

  async function handleResetDb() {
    if (!onResetGlobalDatabase) return;
    resettingDb = true;
    try {
      await onResetGlobalDatabase();
      confirmingReset = false;
    } finally {
      resettingDb = false;
    }
  }
</script>

<header
  id="titlebar"
  data-tauri-drag-region
  class="h-10 shrink-0 bg-gray-900 flex items-center justify-between select-none z-50 px-2 border-b border-gray-800/80"
>
  <!-- Left: App Icon & Title + Runes/Builds Tabs -->
  <div class="flex items-center gap-3.5 h-full" data-tauri-drag-region>
    <div class="flex items-center gap-2 px-1 h-full pointer-events-none" data-tauri-drag-region>
      <img
        src="/src/assets/logo.png"
        alt="LCU Companion"
        class="h-5 w-5 object-contain pointer-events-none drop-shadow-sm"
      />
      <span class="text-xs sm:text-sm font-bold text-gray-200 tracking-wide pointer-events-none">
        LCU Companion
      </span>
    </div>

    <!-- Vertical separator -->
    <div class="h-4 w-px bg-gray-800 pointer-events-none"></div>

    <!-- Runes and Builds navigation tabs -->
    <NavTabs {activeTab} {onTabChange} />
  </div>

  <!-- Right: Background Operations, Logs, Settings, Window Controls -->
  <div class="flex items-center h-full">
    <!-- Active Operations Indicator (Transparent with Pulsing Dot) -->
    {#if agentOperations.length > 0}
      <div class="relative group h-full flex items-center">
        <button
          type="button"
          class="h-full px-2.5 text-cyan-400 hover:text-cyan-200 hover:bg-gray-800/80 transition-colors flex items-center justify-center cursor-pointer"
          title={agentOperations.map((o) => `${o.message}${o.current !== null && o.total !== null ? ` (${o.current}/${o.total})` : ''}`).join('\n')}
          aria-label="Active background operations"
        >
          <span class="relative flex h-2.5 w-2.5">
            <span class="absolute inline-flex h-full w-full animate-ping rounded-full bg-cyan-400 opacity-75"></span>
            <span class="relative inline-flex h-2.5 w-2.5 rounded-full bg-cyan-500"></span>
          </span>
        </button>

        <!-- Dropdown on hover -->
        <div class="pointer-events-none absolute right-0 top-10 z-50 hidden w-64 rounded-lg border border-cyan-800/80 bg-gray-950/95 p-2.5 text-[11px] text-cyan-100 shadow-2xl backdrop-blur group-hover:block">
          <div class="font-semibold text-cyan-300 mb-1 flex items-center justify-between">
            <span>Active Operations</span>
            <span class="text-[10px] text-cyan-400">{agentOperations.length}</span>
          </div>
          {#each agentOperations as operation (operation.id)}
            <div class="py-1 border-t border-gray-800">
              <div class="flex items-center justify-between gap-2">
                <span class="truncate">{operation.message}</span>
                {#if operation.current !== null && operation.total !== null}
                  <span class="tabular-nums text-cyan-300 shrink-0">{operation.current}/{operation.total}</span>
                {/if}
              </div>
              {#if operation.current !== null && operation.total !== null && operation.total > 0}
                <div class="mt-1 h-1 overflow-hidden rounded-full bg-cyan-950">
                  <div
                    class="h-full rounded-full bg-cyan-400 transition-[width] duration-200"
                    style={`width: ${Math.min(100, Math.round((operation.current / operation.total) * 100))}%`}
                  ></div>
                </div>
              {/if}
            </div>
          {/each}
        </div>
      </div>
    {/if}

    <div class="flex items-center gap-2 px-2">
      <!-- Hide Low Pick-Rate Filter Toggle -->
      <button
        type="button"
        onclick={onToggleHideLowPickrate}
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

      <!-- LCU Client Status Indicator -->
      {#if isLcuConnected}
        <span class="inline-flex items-center gap-1.5 rounded-full border border-emerald-500/30 bg-emerald-950/60 px-2.5 py-0.5 text-[11px] font-medium text-emerald-300 select-none">
          <span class="h-1.5 w-1.5 rounded-full bg-emerald-400"></span>
          LCU API: Connected
        </span>
      {:else}
        <span class="inline-flex items-center gap-1.5 rounded-full border border-rose-500/30 bg-rose-950/60 px-2.5 py-0.5 text-[11px] font-medium text-rose-300 select-none">
          <span class="h-1.5 w-1.5 rounded-full bg-rose-400"></span>
          LCU API: Offline
        </span>
      {/if}
    </div>

    <!-- Logs Modal Button -->
    <button
      type="button"
      onclick={() => (logsModalOpen = true)}
      class="h-full px-2.5 text-gray-400 hover:text-white hover:bg-gray-800/80 transition-colors flex items-center gap-1.5 text-xs font-medium cursor-pointer"
      title="View Agent & Application Logs"
    >
      <span class="icon-[heroicons--command-line] text-sm text-amber-400/90"></span>
      <span class="hidden sm:inline">Logs</span>
    </button>

    <!-- Global Database Settings Cog Button -->
    <div class="relative h-full flex items-center">
      <button
        type="button"
        onclick={() => (settingsOpen = !settingsOpen)}
        class="h-full px-2.5 text-gray-400 hover:text-white hover:bg-gray-800/80 transition-colors flex items-center justify-center cursor-pointer relative"
        title="Global Database Settings & Crawler Controls"
        aria-label="Global database settings"
      >
        <span class="icon-[heroicons--cog-6-tooth] text-base"></span>
        {#if globalHasPatchMismatch}
          <span class="absolute right-1.5 top-2 flex h-2 w-2">
            <span class="absolute inline-flex h-full w-full animate-ping rounded-full bg-amber-400 opacity-75"></span>
            <span class="relative inline-flex h-2 w-2 rounded-full bg-amber-500"></span>
          </span>
        {/if}
      </button>

      <!-- Settings Dropdown Popover -->
      {#if settingsOpen}
        <div class="absolute right-0 top-10 z-50 w-80 rounded-xl border border-gray-700 bg-gray-900/95 p-3.5 shadow-2xl backdrop-blur">
          <div class="flex items-center justify-between pb-2 mb-2 border-b border-gray-800">
            <div class="flex items-center gap-2">
              <span class="icon-[heroicons--server-stack] text-cyan-400 text-sm"></span>
              <span class="text-xs font-bold uppercase tracking-wider text-white">Global Crawler & DB</span>
            </div>
            <button
              type="button"
              aria-label="Close settings"
              onclick={() => (settingsOpen = false)}
              class="p-0.5 text-gray-400 hover:text-white transition-colors"
            >
              <span class="icon-[heroicons--x-mark] text-sm"></span>
            </button>
          </div>

          <!-- Outdated Patch Notice -->
          {#if globalHasPatchMismatch}
            <div class="mb-3 rounded-lg border border-amber-500/40 bg-amber-950/40 p-2 text-[11px] text-amber-200">
              <div class="flex items-center gap-1.5 font-semibold text-amber-300">
                <span class="icon-[heroicons--exclamation-triangle] text-sm shrink-0 text-amber-400"></span>
                <span>Cross-patch games detected</span>
              </div>
              <p class="mt-1 text-[10px] leading-relaxed text-amber-200/80">
                {#if globalCurrentPatch}
                  Current patch is <span class="font-semibold text-amber-100">v{globalCurrentPatch}</span>, but {globalOutdatedGames.toLocaleString()} stored games are from older patches.
                {:else}
                  Stored games span multiple patches ({globalOutdatedGames.toLocaleString()} from older patches).
                {/if}
              </p>
              {#if globalPatches.length > 0}
                <div class="mt-1.5 flex flex-wrap gap-1">
                  {#each globalPatches as entry (entry.patch)}
                    <span class="rounded border px-1.5 py-0.5 text-[9px] tabular-nums {globalCurrentPatch && entry.patch === globalCurrentPatch ? 'border-emerald-600/50 bg-emerald-950/50 text-emerald-300' : 'border-amber-600/40 bg-amber-950/60 text-amber-300'}">
                      {entry.patch === 'unknown' ? 'unknown' : `v${entry.patch}`}: {entry.games.toLocaleString()}
                    </span>
                  {/each}
                </div>
              {/if}
            </div>
          {/if}

          <!-- Crawler Worker Controls -->
          <div class="mb-3 p-2.5 rounded-lg bg-gray-950/70 border border-gray-800 space-y-2">
            <div class="flex items-center justify-between text-xs">
              <span class="text-gray-400 font-medium">Crawler Status</span>
              <span class="font-semibold {crawlerPaused ? 'text-amber-400' : 'text-emerald-400'}">
                {crawlerPaused ? 'Paused' : globalCollectorState}
              </span>
            </div>

            {#if globalQueuedJobs > 0}
              <div class="flex items-center justify-between text-[11px] text-gray-400">
                <span>Queue</span>
                <span class="tabular-nums text-cyan-300">{globalQueuedJobs.toLocaleString()} accounts</span>
              </div>
            {/if}

            <div class="flex items-center justify-between text-[11px] text-gray-400">
              <span>Rate / ETA</span>
              <span class="tabular-nums text-cyan-300">
                {crawlerRateText}{#if crawlerEtaText} · {crawlerEtaText}{/if}
              </span>
            </div>

            <!-- Action Buttons: Pause/Resume and Stop -->
            <div class="flex gap-2 pt-1">
              <button
                type="button"
                disabled={pausingCrawler}
                onclick={handleTogglePause}
                class="flex-1 py-1.5 px-2 rounded text-xs font-semibold flex items-center justify-center gap-1.5 transition-colors border {crawlerPaused ? 'bg-emerald-950/50 hover:bg-emerald-900/60 text-emerald-300 border-emerald-700/60' : 'bg-amber-950/50 hover:bg-amber-900/60 text-amber-300 border-amber-700/60'}"
              >
                <span class="icon-[heroicons--{crawlerPaused ? 'play' : 'pause'}] text-sm"></span>
                {crawlerPaused ? 'Resume' : 'Pause'}
              </button>

              <button
                type="button"
                disabled={stoppingCrawler}
                onclick={handleStopCrawler}
                class="py-1.5 px-3 rounded text-xs font-semibold flex items-center justify-center gap-1.5 transition-colors border bg-gray-800 hover:bg-gray-700 text-gray-300 border-gray-700 hover:text-white"
                title="Stop crawler by clamping game limit to current collected total"
              >
                <span class="icon-[heroicons--stop] text-sm text-red-400"></span>
                Stop
              </button>
            </div>
          </div>

          <!-- Limit setting -->
          <div class="space-y-1.5 mb-3">
            <div class="flex items-center justify-between">
              <label class="text-[11px] uppercase tracking-wider text-gray-400 font-semibold" for="titlebar-global-aram-limit">
                Game Collection Limit
              </label>
              <span class="text-[10px] tabular-nums text-gray-400">
                {globalGames.toLocaleString()} / {globalLimit.toLocaleString()}
              </span>
            </div>
            <input
              id="titlebar-global-aram-limit"
              type="number"
              min="1"
              max="100000"
              bind:value={localLimit}
              class="w-full rounded border border-gray-700 bg-gray-950 px-2.5 py-1.5 text-xs text-gray-100 focus:outline-none focus:border-cyan-500"
            />
            <button
              type="button"
              disabled={savingLimit || resettingDb}
              onclick={handleSaveLimit}
              class="w-full mt-1.5 py-1.5 rounded bg-cyan-600 hover:bg-cyan-500 text-white text-xs font-semibold transition-colors disabled:opacity-50"
            >
              {savingLimit ? 'Saving...' : 'Save Limit'}
            </button>
          </div>

          <!-- Database Reset Section -->
          <div class="border-t border-gray-800 pt-2.5">
            {#if !confirmingReset}
              <button
                type="button"
                disabled={resettingDb || savingLimit}
                onclick={() => (confirmingReset = true)}
                class="w-full py-1.5 px-2 rounded border border-red-900/60 bg-red-950/20 text-red-400 hover:bg-red-950/50 hover:text-red-300 text-xs font-medium transition-colors flex items-center justify-center gap-1.5"
              >
                <span class="icon-[heroicons--trash] text-sm"></span>
                Reset Database & Search Tree
              </button>
            {:else}
              <p class="mb-2 text-[10px] text-red-300 leading-snug">
                Delete all {globalGames.toLocaleString()} global games and wipe account queue?
              </p>
              <div class="flex gap-2">
                <button
                  type="button"
                  disabled={resettingDb}
                  onclick={handleResetDb}
                  class="flex-1 py-1.5 rounded bg-red-600 hover:bg-red-500 text-white text-xs font-semibold transition-colors disabled:opacity-50"
                >
                  {resettingDb ? 'Resetting...' : 'Confirm'}
                </button>
                <button
                  type="button"
                  disabled={resettingDb}
                  onclick={() => (confirmingReset = false)}
                  class="px-3 py-1.5 rounded border border-gray-700 bg-gray-800 text-gray-300 hover:bg-gray-700 text-xs transition-colors"
                >
                  Cancel
                </button>
              </div>
            {/if}
          </div>
        </div>
      {/if}
    </div>

    <!-- Window Action Buttons -->
    <button
      type="button"
      onclick={minimize}
      class="h-full px-3 text-gray-400 hover:text-white hover:bg-gray-800/80 focus:outline-none transition-colors inline-flex items-center justify-center cursor-pointer"
      title="Minimize"
    >
      <span class="icon-[heroicons--minus] h-4 w-4"></span>
    </button>
    <button
      type="button"
      onclick={toggleMaximize}
      class="h-full px-3 text-gray-400 hover:text-white hover:bg-gray-800/80 focus:outline-none transition-colors inline-flex items-center justify-center cursor-pointer"
      title="Maximize"
    >
      <span class="icon-[heroicons--stop] h-3.5 w-3.5"></span>
    </button>
    <button
      type="button"
      onclick={close}
      class="h-full px-3 text-gray-400 hover:text-white hover:bg-red-600 focus:outline-none transition-colors inline-flex items-center justify-center cursor-pointer"
      title="Close"
    >
      <span class="icon-[heroicons--x-mark] h-4.5 w-4.5"></span>
    </button>
  </div>
</header>

<!-- Logs Modal -->
<LogsModal isOpen={logsModalOpen} onClose={() => (logsModalOpen = false)} />
