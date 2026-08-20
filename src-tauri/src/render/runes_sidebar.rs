use crate::models::{ClientRunePageSummary, RuneSetSummary};
use crate::render::theme::RuneTheme;
use crate::services::rune_tree::{get_active_companion_rune_name, get_style_info};

fn escape_html(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#x27;")
}

pub async fn render_runes_sidebar(
    client_pages: &[ClientRunePageSummary],
    saved_runesets: &[RuneSetSummary],
    selected_key: Option<&str>,
    error: Option<&str>,
) -> String {
    let active_companion_name = get_active_companion_rune_name().await;

    let mut resolved_selected = selected_key.map(|s| s.to_string());
    if resolved_selected.is_none() {
        if let Some(first_saved) = saved_runesets.first() {
            resolved_selected = Some(format!("saved:{}", first_saved.id));
        } else if let Some(active) = client_pages.iter().find(|p| p.is_active) {
            resolved_selected = Some(format!("client:{}", active.id));
        } else if let Some(first_client) = client_pages.first() {
            resolved_selected = Some(format!("client:{}", first_client.id));
        }
    }

    let current_key = resolved_selected.as_deref().unwrap_or("");

    // 1. Companion Saved Runes Section (rendered first)
    let mut saved_items = Vec::new();
    for item in saved_runesets {
        let item_key = format!("saved:{}", item.id);
        let is_selected = item_key == current_key;
        let is_active = active_companion_name.as_deref() == Some(&item.rune_page_name);
        let theme = RuneTheme::for_style(item.primary_style_id);
        let name_escaped = escape_html(&item.rune_page_name);
        let (style_name, style_icon_url) = get_style_info(item.primary_style_id).await;
        let style_or_source = if !style_name.is_empty() {
            style_name.clone()
        } else {
            item.source.clone()
        };
        let created_date = if item.created_at.len() >= 10 {
            &item.created_at[..10]
        } else {
            &item.created_at
        };
        let runeset_id_str = item.id.to_string();

        let active_badge = if is_active {
            r##"<span class="inline-flex items-center gap-1 rounded-full border border-purple-500/40 bg-purple-500/15 px-2 py-0.5 text-[10px] font-bold uppercase tracking-wider text-purple-300"><span class="h-1.5 w-1.5 rounded-full bg-purple-400 animate-pulse"></span>Active</span>"##
        } else {
            ""
        };

        let icon_img = if !style_icon_url.is_empty() {
            let img_tpl = r##"<img src="{{icon_url}}" alt="{{style_name}}" class="h-full w-full object-contain">"##;
            img_tpl
                .replace("{{icon_url}}", &escape_html(&style_icon_url))
                .replace("{{style_name}}", &escape_html(&style_name))
        } else {
            r##"<div class="h-full w-full rounded-lg bg-gray-800"></div>"##.to_string()
        };

        let (card_classes, icon_box_classes, title_classes, indicator_line) = if is_selected {
            (
                format!(
                    "group relative flex items-center justify-between gap-3 rounded-2xl border-2 {} bg-gradient-to-r {} p-3 ring-1 {} shadow-md {} cursor-pointer",
                    theme.border_primary, theme.card_gradient, theme.border_subtle, theme.shadow_glow
                ),
                format!(
                    "relative flex h-10 w-10 shrink-0 items-center justify-center rounded-xl border p-1.5 shadow-sm ring-1 {} {}",
                    theme.ring_active, theme.icon_box_classes
                ),
                format!("truncate text-sm font-bold {}", theme.text_accent),
                format!(r##"<div class="absolute -left-1 top-1/2 -translate-y-1/2 h-6 w-1 rounded-r-full {}"></div>"##, theme.active_bar),
            )
        } else {
            (
                "group relative flex items-center justify-between gap-3 rounded-2xl border border-gray-800/80 bg-gray-900/70 p-3 transition-all duration-200 hover:border-gray-700 hover:bg-gray-800/80 hover:shadow-md hover:shadow-black/30 cursor-pointer".to_string(),
                format!(
                    "relative flex h-10 w-10 shrink-0 items-center justify-center rounded-xl border p-1.5 shadow-sm group-hover:brightness-110 transition {}",
                    theme.icon_box_classes
                ),
                "truncate text-sm font-semibold text-gray-200 group-hover:text-white transition".to_string(),
                "".to_string(),
            )
        };

        let style_badge_html = format!(
            r##"<span class="inline-flex items-center rounded-md px-1.5 py-0.5 text-[10px] font-semibold {}">{}</span>"##,
            theme.badge_classes,
            escape_html(&style_or_source)
        );

        let saved_template = r##"<li class="{{card_classes}}">
    {{indicator_line}}
    <button
        type="button"
        hx-get="command:ui_rune_tree_saved"
        hx-vals='{"runesetId": {{runeset_id}}}'
        hx-target="#current-rune-tree-container"
        hx-swap="outerHTML"
        class="flex min-w-0 flex-1 items-center gap-3 text-left focus:outline-none cursor-pointer"
    >
        <div class="{{icon_box_classes}}">
            {{icon_img}}
        </div>
        <div class="min-w-0 flex-1">
            <div class="flex items-center gap-2">
                <p class="{{title_classes}}">{{name_escaped}}</p>
                {{active_badge}}
            </div>
            <div class="flex items-center gap-2 mt-0.5">
                {{style_badge_html}}
                <span class="text-[11px] text-gray-500">{{created_date}}</span>
            </div>
        </div>
    </button>
    <button
        type="button"
        title="Delete from database"
        hx-post="command:ui_delete_runeset"
        hx-vals='{"runesetId": {{runeset_id}}}'
        hx-target="#saved-runes-container"
        hx-swap="outerHTML"
        hx-confirm="Delete saved rune page ({{name_escaped}})?"
        class="inline-flex shrink-0 items-center justify-center rounded-xl p-2 text-gray-500 opacity-0 transition-all hover:bg-red-500/10 hover:text-red-400 hover:border-red-500/30 border border-transparent focus:outline-none group-hover:opacity-100 cursor-pointer"
    >
        <svg class="h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"/></svg>
    </button>
</li>"##;

        saved_items.push(
            saved_template
                .replace("{{card_classes}}", &card_classes)
                .replace("{{indicator_line}}", &indicator_line)
                .replace("{{runeset_id}}", &runeset_id_str)
                .replace("{{icon_box_classes}}", &icon_box_classes)
                .replace("{{icon_img}}", &icon_img)
                .replace("{{title_classes}}", &title_classes)
                .replace("{{name_escaped}}", &name_escaped)
                .replace("{{active_badge}}", active_badge)
                .replace("{{style_badge_html}}", &style_badge_html)
                .replace("{{created_date}}", &escape_html(created_date)),
        );
    }

    let saved_content = if saved_items.is_empty() {
        r##"<div class="flex flex-col items-center justify-center rounded-2xl border border-dashed border-gray-800 bg-gray-900/30 p-5 text-center">
            <div class="mb-2 flex h-8 w-8 items-center justify-center rounded-full bg-purple-950/40 text-purple-400 border border-purple-900/40">
                <svg class="h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 4H6a2 2 0 00-2 2v12a2 2 0 002 2h12a2 2 0 002-2V6a2 2 0 00-2-2h-2m-4-1v8m0 0l3-3m-3 3L9 8m-5 5h2.586a1 1 0 01.707.293l2.414 2.414a1 1 0 00.707.293h3.172a1 1 0 00.707-.293l2.414-2.414a1 1 0 01.707-.293H20"/></svg>
            </div>
            <p class="text-xs font-medium text-gray-300">No companion pages saved</p>
            <p class="mt-1 text-[11px] text-gray-500">Click <span class="font-medium text-purple-400">Import</span> on any client page to save it</p>
        </div>"##.to_string()
    } else {
        format!(r##"<ul class="space-y-2">{}</ul>"##, saved_items.join(""))
    };

    // 2. League Client Pages Section (rendered second)
    let mut client_items = Vec::new();
    for page in client_pages {
        let item_key = format!("client:{}", page.id);
        let is_selected = item_key == current_key;
        let theme = RuneTheme::for_style(page.primary_style_id);
        let name_escaped = escape_html(&page.name);
        let style_escaped = escape_html(&page.primary_style_name);
        let page_id_str = page.id.to_string();

        let active_badge = if page.is_active {
            r##"<span class="inline-flex items-center gap-1 rounded-full border border-emerald-500/30 bg-emerald-500/10 px-2 py-0.5 text-[10px] font-semibold text-emerald-400">
                <span class="h-1.5 w-1.5 rounded-full bg-emerald-400 animate-pulse"></span>
                Active
            </span>"##
        } else {
            ""
        };

        let icon_img = if !page.primary_style_icon_url.is_empty() {
            let img_tpl = r##"<img src="{{icon_url}}" alt="{{style_escaped}}" class="h-full w-full object-contain">"##;
            img_tpl
                .replace("{{icon_url}}", &escape_html(&page.primary_style_icon_url))
                .replace("{{style_escaped}}", &style_escaped)
        } else {
            r##"<div class="h-full w-full rounded-lg bg-gray-800"></div>"##.to_string()
        };

        let (card_classes, icon_box_classes, title_classes, indicator_line) = if is_selected {
            (
                format!(
                    "group relative flex items-center justify-between gap-3 rounded-2xl border-2 {} bg-gradient-to-r {} p-3 ring-1 {} shadow-md {} cursor-pointer",
                    theme.border_primary, theme.card_gradient, theme.border_subtle, theme.shadow_glow
                ),
                format!(
                    "relative flex h-10 w-10 shrink-0 items-center justify-center rounded-xl border p-1.5 shadow-sm ring-1 {} {}",
                    theme.ring_active, theme.icon_box_classes
                ),
                format!("truncate text-sm font-bold {}", theme.text_accent),
                format!(r##"<div class="absolute -left-1 top-1/2 -translate-y-1/2 h-6 w-1 rounded-r-full {}"></div>"##, theme.active_bar),
            )
        } else {
            (
                "group relative flex items-center justify-between gap-3 rounded-2xl border border-gray-800/80 bg-gray-900/70 p-3 transition-all duration-200 hover:border-gray-700 hover:bg-gray-800/80 hover:shadow-md hover:shadow-black/30 cursor-pointer".to_string(),
                format!(
                    "relative flex h-10 w-10 shrink-0 items-center justify-center rounded-xl border p-1.5 shadow-sm group-hover:brightness-110 transition {}",
                    theme.icon_box_classes
                ),
                "truncate text-sm font-semibold text-gray-200 group-hover:text-white transition".to_string(),
                "".to_string(),
            )
        };

        let style_badge_html = format!(
            r##"<span class="inline-flex items-center rounded-md px-1.5 py-0.5 text-[10px] font-semibold {}">{}</span>"##,
            theme.badge_classes,
            style_escaped
        );

        let item_template = r##"<li class="{{card_classes}}">
    {{indicator_line}}
    <button
        type="button"
        hx-get="command:ui_rune_tree_client"
        hx-vals='{"pageId": {{page_id}}}'
        hx-target="#current-rune-tree-container"
        hx-swap="outerHTML"
        class="flex min-w-0 flex-1 items-center gap-3 text-left focus:outline-none cursor-pointer"
    >
        <div class="{{icon_box_classes}}">
            {{icon_img}}
        </div>
        <div class="min-w-0 flex-1">
            <div class="flex items-center gap-2">
                <p class="{{title_classes}}">{{name_escaped}}</p>
                {{active_badge}}
            </div>
            <div class="mt-0.5">
                {{style_badge_html}}
            </div>
        </div>
    </button>
    <button
        type="button"
        title="Import into Companion database"
        hx-post="command:ui_import_client_page"
        hx-vals='{"pageId": {{page_id}}}'
        hx-target="#saved-runes-container"
        hx-swap="outerHTML"
        class="inline-flex shrink-0 items-center gap-1.5 rounded-xl border border-purple-500/30 bg-purple-500/10 px-2.5 py-1.5 text-xs font-semibold text-purple-300 transition-all hover:border-purple-500 hover:bg-purple-600 hover:text-white focus:outline-none focus:ring-2 focus:ring-purple-500/40 shadow-sm cursor-pointer"
    >
        <svg class="h-3.5 w-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-4l-4 4m0 0l-4-4m4 4V4"/></svg>
        <span>Import</span>
    </button>
</li>"##;

        client_items.push(
            item_template
                .replace("{{card_classes}}", &card_classes)
                .replace("{{indicator_line}}", &indicator_line)
                .replace("{{page_id}}", &page_id_str)
                .replace("{{icon_box_classes}}", &icon_box_classes)
                .replace("{{icon_img}}", &icon_img)
                .replace("{{title_classes}}", &title_classes)
                .replace("{{name_escaped}}", &name_escaped)
                .replace("{{active_badge}}", active_badge)
                .replace("{{style_badge_html}}", &style_badge_html),
        );
    }

    let client_content = if client_items.is_empty() {
        r##"<div class="flex flex-col items-center justify-center rounded-2xl border border-dashed border-gray-800 bg-gray-900/30 p-5 text-center">
            <div class="mb-2 flex h-8 w-8 items-center justify-center rounded-full bg-gray-800/80 text-gray-500">
                <svg class="h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"/></svg>
            </div>
            <p class="text-xs font-medium text-gray-400">No client pages detected</p>
            <p class="mt-1 text-[11px] text-gray-500">Launch League of Legends to view pages</p>
        </div>"##.to_string()
    } else {
        format!(r##"<ul class="space-y-2">{}</ul>"##, client_items.join(""))
    };

    let error_html = if let Some(err) = error {
        let err_tpl = r##"<div class="rounded-xl border border-red-500/30 bg-red-500/10 p-3 text-xs text-red-300 mb-4">{{error}}</div>"##;
        err_tpl.replace("{{error}}", &escape_html(err))
    } else {
        "".to_string()
    };

    let template = r##"<div id="saved-runes-container" class="space-y-6">
    {{error_html}}
    <div>
        <div class="mb-3 flex items-center justify-between px-1">
            <div class="flex items-center gap-2">
                <span class="flex h-2 w-2 rounded-full bg-purple-500 shadow-sm shadow-purple-500"></span>
                <h3 class="text-xs font-bold uppercase tracking-wider text-gray-300">Companion Saved Runes</h3>
            </div>
            <span class="rounded-full bg-gray-800 px-2 py-0.5 text-[11px] font-semibold text-gray-400 border border-gray-700/60">{{saved_count}}</span>
        </div>
        {{saved_content}}
    </div>
    <div>
        <div class="mb-3 flex items-center justify-between px-1">
            <div class="flex items-center gap-2">
                <span class="flex h-2 w-2 rounded-full bg-blue-500 shadow-sm shadow-blue-500"></span>
                <h3 class="text-xs font-bold uppercase tracking-wider text-gray-300">League Client Pages</h3>
            </div>
            <span class="rounded-full bg-gray-800 px-2 py-0.5 text-[11px] font-semibold text-gray-400 border border-gray-700/60">{{client_count}}</span>
        </div>
        {{client_content}}
    </div>
</div>"##;

    template
        .replace("{{error_html}}", &error_html)
        .replace("{{saved_count}}", &saved_runesets.len().to_string())
        .replace("{{saved_content}}", &saved_content)
        .replace("{{client_count}}", &client_pages.len().to_string())
        .replace("{{client_content}}", &client_content)
}
