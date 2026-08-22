<script lang="ts">
  import KeystoneCard from "./KeystoneCard.svelte";
  import type { EnrichedKeystoneOption } from "../../types/rune_builder";

  let {
    keystones = [],
    onSelectKeystone,
  }: {
    keystones: EnrichedKeystoneOption[];
    onSelectKeystone: (keystoneId: number) => void;
  } = $props();
</script>

<div class="flex flex-col gap-2 rounded-2xl border border-gray-800 bg-gray-950/70 p-4 shadow-sm">
  <div class="flex items-center justify-between px-1">
    <div class="flex items-center gap-2">
      <span class="flex h-2 w-2 rounded-full bg-purple-400"></span>
      <h4 class="text-xs font-bold uppercase tracking-wider text-gray-300">
        Keystone Options
      </h4>
    </div>
    <span class="text-[11px] text-gray-500 font-medium">
      Ranked by match frequency & winrate delta
    </span>
  </div>

  <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-3 pt-1">
    {#each keystones as keystone (keystone.id)}
      <KeystoneCard {keystone} onSelect={onSelectKeystone} />
    {/each}
  </div>
</div>
