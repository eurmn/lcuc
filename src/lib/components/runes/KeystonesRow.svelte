<script lang="ts">
  import KeystoneCard from "./KeystoneCard.svelte";
  import type { EnrichedKeystoneOption } from "../../types/rune_builder";
  import type { RuneTheme } from "../../utils/theme";

  let {
    visibleKeystones = [],
    selectedKeystoneId = null,
    selectedPerkIds = [],
    theme,
    isEditing = true,
    onSelectKeystone,
  }: {
    visibleKeystones: EnrichedKeystoneOption[];
    selectedKeystoneId: number | null;
    selectedPerkIds: number[];
    theme: RuneTheme;
    isEditing?: boolean;
    onSelectKeystone: (keystoneId: number) => void;
  } = $props();
</script>

<div class="flex flex-col gap-2">
  <div class="flex items-center justify-between px-0.5">
    <span class="text-[11px] font-extrabold uppercase tracking-wider {theme.textAccent}">
      {theme.name} Keystones
    </span>
  </div>

  <div
    class="grid gap-2 w-full min-w-0"
    style="grid-template-columns: repeat({visibleKeystones.length}, minmax(0, 1fr));"
  >
    {#each visibleKeystones as keystone (keystone.id)}
      <KeystoneCard
        keystone={{
          ...keystone,
          selected: (selectedKeystoneId === keystone.id) || selectedPerkIds[0] === keystone.id,
        }}
        {isEditing}
        onSelect={(id) => isEditing && onSelectKeystone(id)}
      />
    {/each}
  </div>
</div>
