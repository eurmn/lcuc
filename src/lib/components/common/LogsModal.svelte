<script lang="ts">
  import { onDestroy, tick } from 'svelte';
  import { listen, type UnlistenFn } from '@tauri-apps/api/event';
  import { getLogs, type LogEntry } from '../../api/agent';

  interface Props {
    isOpen?: boolean;
    onClose: () => void;
  }

  let { isOpen = false, onClose }: Props = $props();

  let logs = $state<LogEntry[]>([]);
  let filterLevel = $state<'ALL' | 'INFO' | 'WARN' | 'ERROR' | 'DEBUG'>('ALL');
  let searchQuery = $state('');
  let autoScroll = $state(true);
  let copied = $state(false);
  let logContainer = $state<HTMLDivElement | null>(null);

  let unlistenFn: UnlistenFn | null = null;

  async function loadInitialLogs() {
    try {
      const initial = await getLogs(300);
      logs = initial;
      if (autoScroll) {
        await tick();
        scrollToBottom();
      }
    } catch (e) {
      console.error('Failed to load logs:', e);
    }
  }

  function scrollToBottom() {
    if (logContainer) {
      logContainer.scrollTop = logContainer.scrollHeight;
    }
  }

  $effect(() => {
    if (isOpen) {
      loadInitialLogs();
      setupEventListener();
    } else {
      cleanupEventListener();
    }
  });

  async function setupEventListener() {
    cleanupEventListener();
    try {
      unlistenFn = await listen<LogEntry>('log_entry', async (event) => {
        logs = [...logs.slice(-500), event.payload];
        if (autoScroll) {
          await tick();
          scrollToBottom();
        }
      });
    } catch (e) {
      console.error('Failed to listen to log_entry event:', e);
    }
  }

  function cleanupEventListener() {
    if (unlistenFn) {
      unlistenFn();
      unlistenFn = null;
    }
  }

  onDestroy(() => {
    cleanupEventListener();
  });

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape' && isOpen) {
      onClose();
    }
  }

  const filteredLogs = $derived(
    logs.filter((entry) => {
      if (filterLevel !== 'ALL' && entry.level.toUpperCase() !== filterLevel) {
        return false;
      }
      if (searchQuery.trim()) {
        const q = searchQuery.toLowerCase();
        const tag = (entry.target || entry.source || '').toLowerCase();
        return (
          entry.message.toLowerCase().includes(q) ||
          tag.includes(q) ||
          entry.level.toLowerCase().includes(q)
        );
      }
      return true;
    })
  );

  async function copyLogs() {
    const text = filteredLogs
      .map((l) => `[${l.timestamp}] [${l.level}] [${l.target || l.source || ''}] ${l.message}`)
      .join('\n');
    try {
      await navigator.clipboard.writeText(text);
      copied = true;
      setTimeout(() => (copied = false), 2000);
    } catch (e) {
      console.error('Copy failed:', e);
    }
  }

  function clearLogs() {
    logs = [];
  }

  function getLevelColor(level: string): string {
    const l = level.toUpperCase();
    if (l === 'ERROR') return 'text-red-400 font-semibold';
    if (l === 'WARN') return 'text-amber-400 font-semibold';
    if (l === 'INFO') return 'text-cyan-400';
    if (l === 'DEBUG') return 'text-purple-400';
    return 'text-gray-300';
  }
</script>

<svelte:window onkeydown={handleKeydown} />

{#if isOpen}
  <div
    class="fixed inset-0 z-50 flex items-center justify-center bg-black/75 backdrop-blur-sm p-4 animate-fade-in"
    onclick={(e) => { if (e.target === e.currentTarget) onClose(); }}
    onkeydown={(e) => { if (e.key === 'Escape') onClose(); }}
    role="dialog"
    aria-modal="true"
    tabindex="-1"
  >
    <div
      class="relative w-full max-w-4xl max-h-[85vh] h-[650px] rounded-2xl bg-gray-900 border border-gray-700 shadow-2xl shadow-black/80 flex flex-col overflow-hidden animate-scale-up"
      role="document"
    >
      <!-- Modal Header -->
      <div class="flex items-center justify-between px-5 py-3.5 border-b border-gray-800 bg-gray-900/90">
        <div class="flex items-center gap-2.5">
          <span class="icon-[heroicons--command-line] text-amber-400 text-lg"></span>
          <h3 class="text-sm font-bold uppercase tracking-wider text-white">
            Agent & Application Logs
          </h3>
          <span class="px-2 py-0.5 text-xs font-mono rounded bg-gray-800 text-gray-400 border border-gray-700">
            {filteredLogs.length} events
          </span>
        </div>

        <div class="flex items-center gap-2">
          <!-- Copy Button -->
          <button
            type="button"
            onclick={copyLogs}
            class="px-2.5 py-1 text-xs rounded font-medium transition-colors border flex items-center gap-1.5 {copied ? 'bg-emerald-900/40 text-emerald-400 border-emerald-700' : 'bg-gray-800 text-gray-300 border-gray-700 hover:bg-gray-700 hover:text-white'}"
            title="Copy logs to clipboard"
          >
            <span class="icon-[heroicons--clipboard-document] text-sm"></span>
            {copied ? 'Copied!' : 'Copy'}
          </button>

          <!-- Clear Button -->
          <button
            type="button"
            onclick={clearLogs}
            class="px-2.5 py-1 text-xs rounded font-medium transition-colors border bg-gray-800 text-gray-300 border-gray-700 hover:bg-gray-700 hover:text-white flex items-center gap-1.5"
            title="Clear display logs"
          >
            <span class="icon-[heroicons--trash] text-sm"></span>
            Clear
          </button>

          <!-- Close button -->
          <button
            type="button"
            onclick={onClose}
            class="p-1 rounded text-gray-400 hover:text-white hover:bg-gray-800 transition-colors ml-2"
            title="Close modal"
          >
            <span class="icon-[heroicons--x-mark] text-lg"></span>
          </button>
        </div>
      </div>

      <!-- Controls & Filter Toolbar -->
      <div class="flex flex-wrap items-center justify-between gap-3 px-5 py-2.5 bg-gray-950/60 border-b border-gray-800 text-xs">
        <!-- Level Filters -->
        <div class="flex items-center gap-1 bg-gray-900 p-1 rounded-lg border border-gray-800">
          {#each ['ALL', 'INFO', 'WARN', 'ERROR', 'DEBUG'] as lvl}
            <button
              type="button"
              onclick={() => (filterLevel = lvl as typeof filterLevel)}
              class="px-2.5 py-0.5 rounded text-xs font-medium transition-colors {filterLevel === lvl ? 'bg-amber-500/20 text-amber-300 border border-amber-500/40' : 'text-gray-400 hover:text-gray-200'}"
            >
              {lvl}
            </button>
          {/each}
        </div>

        <!-- Search Input -->
        <div class="relative flex-1 max-w-xs">
          <span class="absolute inset-y-0 left-0 pl-2.5 flex items-center pointer-events-none text-gray-500">
            <span class="icon-[heroicons--magnifying-glass] text-sm"></span>
          </span>
          <input
            type="text"
            bind:value={searchQuery}
            placeholder="Filter logs by keyword..."
            class="w-full pl-8 pr-3 py-1 rounded bg-gray-900 border border-gray-800 text-xs text-gray-200 placeholder-gray-500 focus:outline-none focus:border-amber-500/50"
          />
          {#if searchQuery}
            <button
              type="button"
              aria-label="Clear search"
              onclick={() => (searchQuery = '')}
              class="absolute inset-y-0 right-0 pr-2 flex items-center text-gray-500 hover:text-gray-300"
            >
              <span class="icon-[heroicons--x-mark] text-xs"></span>
            </button>
          {/if}
        </div>

        <!-- Auto-scroll toggle -->
        <label class="flex items-center gap-2 cursor-pointer select-none text-gray-400 hover:text-gray-200">
          <input
            type="checkbox"
            bind:checked={autoScroll}
            class="rounded bg-gray-900 border-gray-700 text-amber-500 focus:ring-0 focus:ring-offset-0"
          />
          <span>Auto-scroll</span>
        </label>
      </div>

      <!-- Logs Console Window -->
      <div
        bind:this={logContainer}
        class="flex-1 overflow-y-auto p-4 font-mono text-xs leading-5 bg-gray-950 select-text space-y-1"
      >
        {#if filteredLogs.length === 0}
          <div class="h-full flex items-center justify-center text-gray-600 italic">
            No log entries match the current filter.
          </div>
        {:else}
          {#each filteredLogs as entry}
            {@const tag = entry.target || entry.source || ''}
            <div class="hover:bg-gray-900/60 rounded px-1.5 py-0.5 flex items-baseline gap-2 group">
              <span class="text-gray-500 shrink-0 select-none text-[10px]">{entry.timestamp}</span>
              <span class="shrink-0 w-12 text-center text-[10px] uppercase font-bold px-1 rounded bg-gray-900/80 border border-gray-800 {getLevelColor(entry.level)}">
                {entry.level}
              </span>
              <span class="text-gray-400 shrink-0 text-[11px] truncate max-w-[140px]" title={tag}>
                [{tag.replace('lcu_companion::', '')}]
              </span>
              <span class="text-gray-200 break-all flex-1 whitespace-pre-wrap">
                {entry.message}
              </span>
            </div>
          {/each}
        {/if}
      </div>
    </div>
  </div>
{/if}
