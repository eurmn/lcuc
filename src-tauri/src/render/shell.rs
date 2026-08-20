//! # Application Shell Render Engine
//!
//! Renders the unified full-height rune companion workspace with the summoner profile card
//! integrated in the left sidebar directly above the rune library.

pub fn render_shell(_tab: &str) -> String {
    r##"<div id="app-shell" class="flex-1 flex overflow-hidden bg-gray-900 text-gray-100 relative selection:bg-purple-600 selection:text-white">
    <!-- Left Sidebar: Profile Card & Rune Pages -->
    <aside class="w-full max-w-sm shrink-0 border-r border-gray-800 bg-gray-900/60 overflow-y-auto h-full">
        <div class="h-full p-4 flex flex-col gap-4">
            <!-- Summoner Profile Card Container -->
            <div
                id="profile-card-container"
                class="w-full rounded-2xl border border-gray-800 bg-gray-900/80 p-3.5"
                hx-get="command:ui_profile_card"
                hx-trigger="load"
                hx-swap="outerHTML"
            >
                <div class="flex animate-pulse items-center gap-3">
                    <div class="h-12 w-12 shrink-0 rounded-full border border-gray-700 bg-gray-800"></div>
                    <div class="min-w-0 flex-1">
                        <div class="h-2.5 w-16 rounded-full bg-gray-800"></div>
                        <div class="mt-1.5 h-3.5 w-24 rounded-full bg-gray-700"></div>
                    </div>
                </div>
            </div>

            <!-- Horizontal Separator -->
            <hr class="border-gray-800" />

            <!-- Saved Runes List Container -->
            <div
                id="saved-runes-container"
                hx-get="command:ui_runesets"
                hx-trigger="load"
                hx-swap="outerHTML"
            >
                <div class="space-y-6 animate-pulse">
                    <div>
                        <div class="mb-3 flex items-center justify-between px-1">
                            <div class="flex items-center gap-2">
                                <span class="flex h-2 w-2 rounded-full bg-gray-700"></span>
                                <div class="h-3 w-28 rounded-full bg-gray-700"></div>
                            </div>
                            <div class="h-4 w-6 rounded-full bg-gray-800"></div>
                        </div>
                        <ul class="space-y-2">
                            <li class="rounded-2xl border border-gray-800 bg-gray-900/60 p-3">
                                <div class="flex items-center gap-3">
                                    <div class="h-10 w-10 rounded-xl bg-gray-800"></div>
                                    <div class="flex-1 min-w-0">
                                        <div class="h-3.5 w-28 rounded-full bg-gray-700"></div>
                                        <div class="mt-2 h-2.5 w-16 rounded-full bg-gray-800"></div>
                                    </div>
                                </div>
                            </li>
                        </ul>
                    </div>
                </div>
            </div>
        </div>
    </aside>

    <!-- Center Rune Canvas -->
    <main class="flex-1 flex items-center justify-center bg-gray-900 p-6 overflow-y-auto h-full relative">
        <!-- Center Rune Tree Section -->
        <div
            id="current-rune-tree-container"
            class="w-full max-w-4xl mx-auto animate-pulse rounded-xl bg-gray-900 p-5 my-auto"
            hx-get="command:ui_rune_tree_current"
            hx-trigger="load"
            hx-swap="outerHTML"
        >
            <div class="mb-4 flex items-center justify-between gap-4 border-b border-gray-800 pb-3">
                <div class="h-5 w-44 rounded-full bg-gray-800"></div>
            </div>
            <div class="h-64 rounded-xl bg-gray-950/80 p-4 border border-gray-800"></div>
        </div>
    </main>
</div>"##.to_string()
}
