<script lang="ts">
  import { getCurrentWindow } from "@tauri-apps/api/window";

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
  class="h-8 shrink-0 bg-gray-950 border-b border-gray-800 flex items-center justify-between select-none z-50"
>
  <!-- Left: App Icon & Title -->
  <div class="flex items-center gap-2 px-3 h-full pointer-events-none" data-tauri-drag-region>
    <img src="/src/assets/logo.png" alt="LCU Companion" class="h-4 w-4 object-contain" />
    <span class="text-xs font-semibold text-gray-200 tracking-wide">LCU Companion</span>
  </div>

  <!-- Right: Window Action Buttons -->
  <div class="flex items-center h-full">
    <button
      type="button"
      onclick={minimize}
      class="h-full px-3 text-xs text-gray-400 hover:text-gray-100 hover:bg-gray-800 focus:outline-none transition-colors inline-flex items-center justify-center cursor-pointer"
      title="Minimize"
    >
      Minimize
    </button>
    <button
      type="button"
      onclick={toggleMaximize}
      class="h-full px-3 text-xs text-gray-400 hover:text-gray-100 hover:bg-gray-800 focus:outline-none transition-colors inline-flex items-center justify-center cursor-pointer"
      title="Maximize"
    >
      Maximize
    </button>
    <button
      type="button"
      onclick={close}
      class="h-full px-3 text-xs text-gray-400 hover:text-white hover:bg-red-600 focus:outline-none transition-colors inline-flex items-center justify-center cursor-pointer"
      title="Close"
    >
      Close
    </button>
  </div>
</header>
