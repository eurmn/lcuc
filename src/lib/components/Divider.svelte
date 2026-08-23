<script lang="ts">
  import { MotionDiv, type MotionTransition } from "@humanspeak/svelte-motion";

  let {
    margin = "0.75rem",
    transition = {
      duration: 0.35,
      ease: [0.16, 1, 0.3, 1],
    },
    class: customClass = "",
  }: {
    margin?: string;
    transition?: MotionTransition;
    class?: string;
  } = $props();
</script>

<MotionDiv
  initial={{ opacity: 0, marginTop: 0, marginBottom: 0 }}
  animate={{ opacity: 1, marginTop: margin, marginBottom: margin }}
  exit={{ opacity: 0, marginTop: 0, marginBottom: 0 }}
  {transition}
  class="h-px w-full bg-gray-800/80 {customClass}"
/>
