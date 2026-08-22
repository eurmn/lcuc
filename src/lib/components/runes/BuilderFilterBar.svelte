<script lang="ts">
  import { LEAGUE_ROLES_MAP } from "../../types/rune_builder";


  let {
    selectedRole = 1,
    selectedTiers = [4, 5, 6, 7],
    pageName = "Custom Rune Page",
    saving = false,
    exporting = false,
    providerName = "Coachless.gg",
    onSelectRole,
    onToggleTier,
    onApplyRecommended,
    onSave,
    onExport,
  }: {
    selectedRole: number;
    selectedTiers: number[];
    pageName: string;
    saving: boolean;
    exporting: boolean;
    providerName?: string;
    onSelectRole: (role: number) => void;
    onToggleTier: (tier: number) => void;
    onApplyRecommended: () => void;
    onSave: () => void;
    onExport: () => void;
  } = $props();

  const roleEntries = Object.entries(LEAGUE_ROLES_MAP).map(([key, val]) => ({
    id: parseInt(key, 10),
    label: val.label,
  }));

  // Supported UI rank tiers: Gold (3) to Master+ (7)
  const tierEntries: { id: number; label: string }[] = [
    { id: 3, label: "Gold" },
    { id: 4, label: "Platinum" },
    { id: 5, label: "Emerald" },
    { id: 6, label: "Diamond" },
    { id: 7, label: "Master+" },
  ];
</script>

<div class="flex flex-col gap-3 rounded-2xl border border-gray-800 bg-gray-950/80 p-4 shadow-md">
  <!-- Top Row: Page Name & Actions -->
  <div class="flex flex-wrap items-center justify-between gap-3 border-b border-gray-800 pb-3">
    <!-- Page Name Input -->
    <div class="flex items-center gap-3 flex-1 min-w-[200px] max-w-md">
      <span class="flex h-2.5 w-2.5 rounded-full bg-purple-500 shadow-sm shadow-purple-500 shrink-0"></span>
      <input
        type="text"
        bind:value={pageName}
        placeholder="Rune build name..."
        class="w-full rounded-xl border border-gray-700 bg-gray-900 px-3.5 py-2 text-sm font-bold text-white placeholder:text-gray-500 focus:border-purple-500 focus:outline-none focus:ring-1 focus:ring-purple-500"
      />
    </div>

    <!-- Action Buttons -->
    <div class="flex items-center gap-2 shrink-0">
      <!-- Apply Recommended Button -->
      <button
        type="button"
        title="Auto-select the most popular & highest winrate combination"
        onclick={onApplyRecommended}
        class="inline-flex items-center gap-1.5 rounded-xl border border-blue-500/40 bg-blue-500/10 px-3 py-2 text-xs font-bold text-blue-300 transition-all hover:border-blue-500 hover:bg-blue-600 hover:text-white focus:outline-none focus:ring-2 focus:ring-blue-500/40 shadow-sm cursor-pointer"
      >
        <svg class="h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 10V3L4 14h7v7l9-11h-7z"/>
        </svg>
        <span>Recommended</span>
      </button>

      <!-- Export to Game Button -->
      <button
        type="button"
        title="Export build directly into League Client active slot"
        disabled={exporting}
        onclick={onExport}
        class="inline-flex items-center gap-1.5 rounded-xl border border-purple-500/40 bg-purple-500/10 px-3 py-2 text-xs font-bold text-purple-300 transition-all hover:border-purple-500 hover:bg-purple-600 hover:text-white focus:outline-none focus:ring-2 focus:ring-purple-500/40 shadow-sm cursor-pointer"
      >
        <svg class="h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-8l-4-4m0 0L8 8m4-4v12"/>
        </svg>
        <span>{exporting ? "Exporting..." : "Export"}</span>
      </button>

      <!-- Save Button -->
      <button
        type="button"
        title="Save this rune build to local Companion SQLite database"
        disabled={saving}
        onclick={onSave}
        class="inline-flex items-center gap-1.5 rounded-xl bg-purple-600 px-4 py-2 text-xs font-bold text-white shadow-md shadow-purple-950/50 transition-all hover:bg-purple-500 focus:outline-none focus:ring-2 focus:ring-purple-500 active:scale-95 cursor-pointer"
      >
        <svg class="h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7"/>
        </svg>
        <span>{saving ? "Saving..." : "Save Page"}</span>
      </button>
    </div>
  </div>

  <!-- Bottom Row: Filters (Role Selector & Rank Multi-Select) -->
  <div class="flex flex-wrap items-center justify-between gap-4">
    <!-- Role Selector Tabs -->
    <div class="flex items-center gap-1">
      <span class="text-[10px] font-bold uppercase tracking-wider text-gray-400 mr-1.5">Role:</span>
      <div class="inline-flex rounded-xl bg-gray-900 p-1 border border-gray-800 shadow-sm">
        {#each roleEntries as role (role.id)}
          <button
            type="button"
            onclick={() => onSelectRole(role.id)}
            class={selectedRole === role.id
              ? "rounded-lg bg-purple-600 px-2.5 py-1 text-xs font-bold text-white shadow-sm cursor-pointer"
              : "rounded-lg px-2.5 py-1 text-xs font-semibold text-gray-400 hover:text-white cursor-pointer transition-colors"}
          >
            {role.label}
          </button>
        {/each}
      </div>
    </div>

    <!-- Rank Tier Multi-Select Filter -->
    <div class="flex items-center gap-1.5">
      <span class="text-[10px] font-bold uppercase tracking-wider text-gray-400 mr-1">Ranks:</span>
      <div class="flex flex-wrap items-center gap-1">
        {#each tierEntries as tier (tier.id)}
          {@const isChecked = selectedTiers.includes(tier.id)}
          <button
            type="button"
            onclick={() => onToggleTier(tier.id)}
            class={isChecked
              ? "inline-flex items-center gap-1 rounded-full border border-purple-500/50 bg-purple-500/20 px-2.5 py-0.5 text-[11px] font-bold text-purple-300 shadow-sm cursor-pointer"
              : "inline-flex items-center gap-1 rounded-full border border-gray-800 bg-gray-900/60 px-2.5 py-0.5 text-[11px] font-medium text-gray-400 hover:border-gray-700 hover:text-gray-300 cursor-pointer"}
          >
            {#if isChecked}
              <svg class="h-3 w-3 text-purple-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="3" d="M5 13l4 4L19 7"/>
              </svg>
            {/if}
            <span>{tier.label}</span>
          </button>
        {/each}
      </div>
    </div>

    <!-- Telemetry Provider Badge -->
    <div class="flex items-center gap-1.5 text-gray-400 text-[11px]">
      <span class="h-1.5 w-1.5 rounded-full bg-emerald-400 animate-pulse"></span>
      <span class="text-[10px] font-medium text-gray-400">Data: <span class="font-bold text-gray-300">{providerName}</span></span>
    </div>
  </div>
</div>
