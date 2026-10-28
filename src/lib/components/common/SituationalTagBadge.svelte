<script lang="ts">
  import { getSituationalTagInfo } from "../../utils/situationalTag";

  let {
    tag,
  }: {
    tag?: string | null;
  } = $props();

  const info = $derived(getSituationalTagInfo(tag));
</script>

{#if info}
  {#if info.iconUrl}
    <span
      class="absolute -top-1 -right-1 flex items-center justify-center w-[18px] h-[18px] rounded-full bg-gray-950/95 border {info.borderClass || 'border-gray-600'} shadow-md p-0.5 z-10 pointer-events-auto"
      title={info.title}
    >
      <img src={info.iconUrl} alt={info.title} class="w-[14px] h-[14px] object-contain" />
    </span>
  {:else}
    <span
      class="absolute -top-1.5 -right-1 px-1 rounded bg-rose-600/95 text-white text-[8px] font-bold tracking-tight shadow pointer-events-none whitespace-nowrap z-10"
    >
      {info.tag}
    </span>
  {/if}
{/if}
