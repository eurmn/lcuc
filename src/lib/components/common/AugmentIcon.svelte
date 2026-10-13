<script lang="ts">
  let {
    src = "",
    name = "",
    rarity = "silver",
    class: className = "h-7 w-7",
    roundedClass = "rounded",
    id = "",
  }: {
    src?: string;
    name?: string;
    rarity?: string;
    class?: string;
    roundedClass?: string;
    id?: string;
  } = $props();

  const normRarity = $derived((rarity ?? "silver").toLowerCase());
  const isPrismatic = $derived(normRarity.includes("prismatic"));
  const isGold = $derived(normRarity.includes("gold"));
</script>

{#if isPrismatic}
  <div
    id={id || undefined}
    class="relative inline-flex items-center justify-center shrink-0 {roundedClass} p-[1px] bg-[linear-gradient(180deg,#e78fff,#70008f)] shadow-sm {className}"
    title={name}
  >
    <div class="flex h-full w-full items-center justify-center {roundedClass === 'rounded-lg' ? 'rounded-[7px]' : 'rounded-[3px]'} bg-gray-950 overflow-hidden">
      {#if src}
        <img src={src} alt={name} class="h-full w-full object-cover" />
      {:else}
        <span class="text-[10px] font-bold text-[#f2c2ff]">{name ? name.slice(0, 1) : "P"}</span>
      {/if}
    </div>
  </div>
{:else if isGold}
  <div
    id={id || undefined}
    class="relative inline-flex items-center justify-center shrink-0 {roundedClass} border border-[#e3c49f] bg-gray-950 overflow-hidden shadow-sm {className}"
    title={name}
  >
    {#if src}
      <img src={src} alt={name} class="h-full w-full object-cover" />
    {:else}
      <span class="text-[10px] font-bold text-[#e3c49f]">{name ? name.slice(0, 1) : "G"}</span>
    {/if}
  </div>
{:else}
  <!-- Silver or default fallback -->
  <div
    id={id || undefined}
    class="relative inline-flex items-center justify-center shrink-0 {roundedClass} border border-[#a3b9bd] bg-gray-950 overflow-hidden shadow-sm {className}"
    title={name}
  >
    {#if src}
      <img src={src} alt={name} class="h-full w-full object-cover" />
    {:else}
      <span class="text-[10px] font-bold text-[#a3b9bd]">{name ? name.slice(0, 1) : "S"}</span>
    {/if}
  </div>
{/if}
