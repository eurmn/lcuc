<script lang="ts">
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import NavTabs from "./navigation/NavTabs.svelte";

  let {
    activeTab = "runes",
    onTabChange,
  }: {
    activeTab?: "runes" | "builds";
    onTabChange?: (tab: "runes" | "builds") => void;
  } = $props();

  const appWindow = getCurrentWindow();

  function minimize() {
    appWindow.minimize();
  }

  function toggleMaximize() {
    appWindow.toggleMaximize();
  }

  function close() {
    appWindow.close();
  }
</script>

<header
  id="titlebar"
  data-tauri-drag-region
  class="h-10 shrink-0 bg-gray-900 flex items-center justify-between select-none z-50 px-2"
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

  <!-- Right: Window Action Buttons with Iconify Icons -->
  <div class="flex items-center h-full">
    <button
      type="button"
      onclick={minimize}
      class="h-full px-3.5 text-gray-400 hover:text-white hover:bg-gray-800/80 focus:outline-none transition-colors inline-flex items-center justify-center cursor-pointer"
      title="Minimize"
    >
      <span class="icon-[heroicons--minus] h-4 w-4"></span>
    </button>
    <button
      type="button"
      onclick={toggleMaximize}
      class="h-full px-3.5 text-gray-400 hover:text-white hover:bg-gray-800/80 focus:outline-none transition-colors inline-flex items-center justify-center cursor-pointer"
      title="Maximize"
    >
      <span class="icon-[heroicons--stop] h-3.5 w-3.5"></span>
    </button>
    <button
      type="button"
      onclick={close}
      class="h-full px-3.5 text-gray-400 hover:text-white hover:bg-red-600 focus:outline-none transition-colors inline-flex items-center justify-center cursor-pointer"
      title="Close"
    >
      <span class="icon-[heroicons--x-mark] h-4.5 w-4.5"></span>
    </button>
  </div>
</header>
