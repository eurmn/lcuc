<script lang="ts">
  export interface NavTabItem {
    id: string;
    label: string;
    icon: string;
    badge?: string;
  }

  let {
    activeTab = "runes",
    tabs = [
      { id: "builds", label: "Builds", icon: "icon-[heroicons--sparkles]" },
      { id: "mayhem", label: "Mayhem", icon: "icon-[heroicons--fire]" },
      { id: "profile", label: "Profile", icon: "icon-[heroicons--user]" },
    ],
    onTabChange,
  }: {
    activeTab?: string;
    tabs?: NavTabItem[];
    onTabChange?: (tab: any) => void;
  } = $props();
</script>

<div
  class="flex items-center gap-1.5 shrink-0 select-none pointer-events-auto"
>
  {#each tabs as tab (tab.id)}
    {@const isActive = activeTab === tab.id}
    <button
      type="button"
      onclick={() => onTabChange?.(tab.id)}
      class="flex items-center justify-center gap-1.5 rounded-lg px-2.5 py-1 text-xs font-semibold transition-all cursor-pointer {isActive
        ? 'bg-purple-600 text-white'
        : 'text-gray-400 hover:bg-gray-800 hover:text-gray-200'}"
      title={tab.label}
    >
      <span class="{tab.icon} h-3.5 w-3.5 shrink-0"></span>
      <span class="truncate">{tab.label}</span>
      {#if tab.badge}
        <span
          class="rounded-full border border-purple-800/60 bg-purple-950/80 px-1.5 py-0.2 text-[9px] font-bold uppercase tracking-wider text-purple-300"
        >
          {tab.badge}
        </span>
      {/if}
    </button>
  {/each}
</div>
