use crate::models::{RuneSlot, RuneStyleTree, RuneTreePayload};
use crate::render::theme::RuneTheme;

fn escape_html(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#x27;")
}

fn render_style_section(
    style: Option<&RuneStyleTree>,
    title: &str,
    is_secondary: bool,
    is_editing: bool,
    _is_saved_page: bool,
    runeset_id_val: &str,
    page_id_val: &str,
    primary_style_id_val: &str,
    sub_style_id_val: &str,
    current_perks_str: &str,
) -> String {
    let tree = match style {
        Some(t) => t,
        None => {
            let empty_tpl = r##"<article class="rounded-2xl border border-gray-800 bg-gray-950/80 p-4 h-full flex flex-col justify-center items-center">
    <h4 class="text-sm font-semibold text-gray-300">{{title}}</h4>
    <p class="mt-2 text-xs text-gray-500">No data</p>
</article>"##;
            return empty_tpl.replace("{{title}}", &escape_html(title));
        }
    };

    let theme = RuneTheme::for_style(Some(tree.id));
    let style_name = escape_html(&tree.name);
    let icon_img = if !tree.icon_url.is_empty() {
        let img_tpl = r##"<img src="{{icon_url}}" alt="{{style_name}}" class="h-6 w-6 object-contain">"##;
        img_tpl
            .replace("{{icon_url}}", &escape_html(&tree.icon_url))
            .replace("{{style_name}}", &style_name)
    } else {
        "".to_string()
    };

    let mut keystone_html = String::new();
    let mut regular_slots_html = Vec::new();

    for (slot_idx, slot) in tree.slots.iter().enumerate() {
        let is_keystone = !is_secondary && slot_idx == 0;
        let selected_opt = slot.options.iter().find(|opt| opt.selected);

        let mut options_html = Vec::new();
        for opt in &slot.options {
            let opt_name = escape_html(&opt.name);
            let opt_id_str = opt.id.to_string();

            if is_editing {
                let slot_type = if is_secondary { "secondary" } else { "primary" };
                let htmx_attrs = format!(
                    r##"hx-post="command:ui_rune_tree_select_perk" hx-include="#rune-page-name-input" hx-vals='{{"runesetId": "{}", "pageId": "{}", "primaryStyleId": "{}", "subStyleId": "{}", "currentPerks": "{}", "clickedPerkId": {}, "slotType": "{}", "slotIndex": {}}}' hx-target="#current-rune-tree-container" hx-swap="outerHTML""##,
                    runeset_id_val, page_id_val, primary_style_id_val, sub_style_id_val, current_perks_str, opt_id_str, slot_type, slot_idx
                );

                if is_keystone {
                    let (btn_classes, check_badge) = if opt.selected {
                        (
                            format!(
                                "group relative inline-flex h-20 w-20 items-center justify-center rounded-full border-2 {} {} p-2.5 ring-4 {} shadow-xl {} scale-105 transition-all cursor-pointer",
                                theme.border_primary, theme.bg_card, theme.ring_focus, theme.shadow_glow
                            ),
                            format!(
                                r##"<span class="absolute -top-1 -right-1 flex h-4 w-4 items-center justify-center rounded-full {} text-white shadow-sm"><svg class="h-3 w-3" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="3" d="M5 13l4 4L19 7"/></svg></span>"##,
                                theme.bg_badge
                            ),
                        )
                    } else {
                        (
                            format!(
                                "group relative inline-flex h-20 w-20 items-center justify-center rounded-full border border-gray-700/80 bg-gray-800/80 p-2.5 opacity-40 hover:opacity-100 {} hover:scale-105 hover:bg-gray-800 transition-all cursor-pointer",
                                theme.border_hover
                            ),
                            "".to_string(),
                        )
                    };

                    let btn_tpl = r##"<button type="button" title="{{opt_name}}" {{htmx_attrs}} class="{{btn_classes}}">
    <img src="{{icon_url}}" alt="{{opt_name}}" class="h-full w-full rounded-full object-cover">
    {{check_badge}}
</button>"##;
                    options_html.push(
                        btn_tpl
                            .replace("{{opt_name}}", &opt_name)
                            .replace("{{htmx_attrs}}", &htmx_attrs)
                            .replace("{{btn_classes}}", &btn_classes)
                            .replace("{{check_badge}}", &check_badge)
                            .replace("{{icon_url}}", &escape_html(&opt.icon_url)),
                    );
                } else {
                    let (btn_classes, check_badge) = if opt.selected {
                        (
                            format!(
                                "group relative inline-flex h-11 w-11 items-center justify-center rounded-full border {} {} p-1 ring-1 {} shadow-md {} scale-105 transition-all cursor-pointer",
                                theme.border_primary, theme.bg_card, theme.ring_active, theme.shadow_glow
                            ),
                            format!(
                                r##"<span class="absolute -top-0.5 -right-0.5 flex h-3.5 w-3.5 items-center justify-center rounded-full {} text-white shadow-sm"><svg class="h-2.5 w-2.5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="3" d="M5 13l4 4L19 7"/></svg></span>"##,
                                theme.bg_badge
                            ),
                        )
                    } else {
                        (
                            format!(
                                "group relative inline-flex h-11 w-11 items-center justify-center rounded-full border border-gray-700/80 bg-gray-800/80 p-1 opacity-40 hover:opacity-100 {} hover:scale-105 hover:bg-gray-700 transition-all cursor-pointer",
                                theme.border_hover
                            ),
                            "".to_string(),
                        )
                    };

                    let btn_tpl = r##"<button type="button" title="{{opt_name}}" {{htmx_attrs}} class="{{btn_classes}}">
    <img src="{{icon_url}}" alt="{{opt_name}}" class="h-full w-full rounded-full object-cover">
    {{check_badge}}
</button>"##;
                    options_html.push(
                        btn_tpl
                            .replace("{{opt_name}}", &opt_name)
                            .replace("{{htmx_attrs}}", &htmx_attrs)
                            .replace("{{btn_classes}}", &btn_classes)
                            .replace("{{check_badge}}", &check_badge)
                            .replace("{{icon_url}}", &escape_html(&opt.icon_url)),
                    );
                }
            } else {
                // View Mode
                if is_keystone {
                    let selected_classes = if opt.selected {
                        format!(
                            "ring-4 {} border-2 {} {} shadow-xl {} scale-105",
                            theme.ring_focus, theme.border_primary, theme.bg_card, theme.shadow_glow
                        )
                    } else {
                        "border-gray-700/60 bg-gray-800/40 opacity-40".to_string()
                    };

                    let elem_tpl = r##"<div title="{{opt_name}}" class="group inline-flex h-20 w-20 items-center justify-center rounded-full border p-2.5 transition-all select-none {{selected_classes}}">
    <img src="{{icon_url}}" alt="{{opt_name}}" class="h-full w-full rounded-full object-cover">
</div>"##;
                    options_html.push(
                        elem_tpl
                            .replace("{{opt_name}}", &opt_name)
                            .replace("{{selected_classes}}", &selected_classes)
                            .replace("{{icon_url}}", &escape_html(&opt.icon_url)),
                    );
                } else {
                    let selected_classes = if opt.selected {
                        format!(
                            "ring-1 {} border {} {} shadow-sm {}",
                            theme.ring_active, theme.border_primary, theme.bg_card, theme.shadow_glow
                        )
                    } else {
                        "border-gray-700/60 bg-gray-800/60 opacity-40".to_string()
                    };

                    let elem_tpl = r##"<div title="{{opt_name}}" class="group inline-flex h-11 w-11 items-center justify-center rounded-full border p-1 transition-all select-none {{selected_classes}}">
    <img src="{{icon_url}}" alt="{{opt_name}}" class="h-full w-full rounded-full object-cover">
</div>"##;
                    options_html.push(
                        elem_tpl
                            .replace("{{opt_name}}", &opt_name)
                            .replace("{{selected_classes}}", &selected_classes)
                            .replace("{{icon_url}}", &escape_html(&opt.icon_url)),
                    );
                }
            }
        }

        let selected_name_html = if let Some(opt) = selected_opt {
            format!(r##"<span class="text-xs font-bold {}">{}</span>"##, theme.text_accent, escape_html(&opt.name))
        } else {
            "".to_string()
        };

        if is_keystone {
            let row_tpl = r##"<div class="mb-6">
    <div class="mb-2 flex items-center justify-between px-1">
        <p class="text-[11px] font-medium uppercase tracking-wide text-gray-400">Keystone</p>
        {{selected_name_html}}
    </div>
    <div class="flex flex-wrap items-center justify-center gap-4 py-2 pb-1">{{options}}</div>
</div>"##;
            keystone_html = row_tpl
                .replace("{{selected_name_html}}", &selected_name_html)
                .replace("{{options}}", &options_html.join(""));
        } else {
            let label = if !slot.label.is_empty() && !slot.label.eq_ignore_ascii_case("slot") {
                escape_html(&slot.label)
            } else {
                format!("Slot {}", slot_idx + 1)
            };

            let slot_card = r##"<div class="rounded-xl border border-gray-800/80 bg-gray-900/90 p-3">
    <div class="mb-2 flex items-center justify-between">
        <p class="text-[11px] font-medium uppercase tracking-wide text-gray-400">{{label}}</p>
        {{selected_name_html}}
    </div>
    <div class="flex flex-wrap items-center justify-center gap-3">{{options}}</div>
</div>"##;
            regular_slots_html.push(
                slot_card
                    .replace("{{label}}", &label)
                    .replace("{{selected_name_html}}", &selected_name_html)
                    .replace("{{options}}", &options_html.join("")),
            );
        }
    }

    let header_block = format!(
        r##"<div class="mb-3.5 flex items-center gap-2.5 shrink-0">
    <div class="flex h-8 w-8 shrink-0 items-center justify-center rounded-xl border {} {} p-1 shadow-sm">
        {}
    </div>
    <div>
        <p class="text-[10px] font-bold uppercase tracking-wider text-gray-400">{}</p>
        <h4 class="text-sm font-bold {}">{}</h4>
    </div>
</div>"##,
        theme.border_subtle, theme.bg_card, icon_img, escape_html(title), theme.text_accent, style_name
    );

    if is_secondary {
        format!(
            r##"<article class="rounded-2xl border {} bg-gray-950/80 p-4 flex flex-col backdrop-blur-sm shadow-md {}">
    {}
    <div class="space-y-3">{}</div>
</article>"##,
            theme.border_subtle,
            theme.shadow_glow,
            header_block,
            regular_slots_html.join("")
        )
    } else {
        format!(
            r##"<article class="rounded-2xl border {} bg-gray-950/80 p-4 h-full flex flex-col backdrop-blur-sm shadow-md {}">
    {}
    <div class="flex-1 flex flex-col justify-center my-auto">
        {}
        <div class="space-y-3">{}</div>
    </div>
</article>"##,
            theme.border_subtle,
            theme.shadow_glow,
            header_block,
            keystone_html,
            regular_slots_html.join("")
        )
    }
}

fn render_stat_mods_section(
    stat_mods: &[RuneSlot],
    is_editing: bool,
    _is_saved_page: bool,
    runeset_id_val: &str,
    page_id_val: &str,
    primary_style_id_val: &str,
    sub_style_id_val: &str,
    current_perks_str: &str,
) -> String {
    let mut blocks = Vec::new();
    for (row_idx, slot) in stat_mods.iter().enumerate() {
        let mut options_html = Vec::new();
        for opt in &slot.options {
            let opt_name = escape_html(&opt.name);
            let opt_id_str = opt.id.to_string();

            if is_editing {
                let htmx_attrs = format!(
                    r##"hx-post="command:ui_rune_tree_select_perk" hx-include="#rune-page-name-input" hx-vals='{{"runesetId": "{}", "pageId": "{}", "primaryStyleId": "{}", "subStyleId": "{}", "currentPerks": "{}", "clickedPerkId": {}, "slotType": "stat", "slotIndex": {}}}' hx-target="#current-rune-tree-container" hx-swap="outerHTML""##,
                    runeset_id_val, page_id_val, primary_style_id_val, sub_style_id_val, current_perks_str, opt_id_str, row_idx
                );

                let btn_classes = if opt.selected {
                    "group relative inline-flex h-8 w-8 items-center justify-center rounded-full border border-purple-400 bg-purple-950/60 p-1 ring-1 ring-purple-400/80 shadow-sm shadow-purple-950/50 scale-105 transition-all cursor-pointer"
                } else {
                    "group relative inline-flex h-8 w-8 items-center justify-center rounded-full border border-transparent bg-gray-800/80 p-1 opacity-40 hover:opacity-100 hover:border-purple-400 hover:scale-105 hover:bg-gray-700 transition-all cursor-pointer"
                };

                let opt_tpl = r##"<button type="button" title="{{opt_name}}" {{htmx_attrs}} class="{{btn_classes}}">
    <img src="{{icon_url}}" alt="{{opt_name}}" class="h-full w-full rounded-full object-cover">
</button>"##;
                options_html.push(
                    opt_tpl
                        .replace("{{opt_name}}", &opt_name)
                        .replace("{{htmx_attrs}}", &htmx_attrs)
                        .replace("{{btn_classes}}", btn_classes)
                        .replace("{{icon_url}}", &escape_html(&opt.icon_url)),
                );
            } else {
                let selected_classes = if opt.selected {
                    "ring-1 ring-purple-400/80 border border-purple-400 bg-purple-950/40 shadow-sm"
                } else {
                    "border-transparent bg-gray-800/40 opacity-40"
                };

                let opt_tpl = r##"<div title="{{opt_name}}" class="inline-flex h-8 w-8 items-center justify-center rounded-full border p-1 transition-all select-none {{selected_classes}}">
    <img src="{{icon_url}}" alt="{{opt_name}}" class="h-full w-full rounded-full object-cover">
</div>"##;
                options_html.push(
                    opt_tpl
                        .replace("{{opt_name}}", &opt_name)
                        .replace("{{selected_classes}}", selected_classes)
                        .replace("{{icon_url}}", &escape_html(&opt.icon_url)),
                );
            }
        }

        let row_tpl = r##"<div class="flex items-center justify-center gap-3 py-1">{{options}}</div>"##;
        blocks.push(row_tpl.replace("{{options}}", &options_html.join("")));
    }

    let section_tpl = r##"<article class="rounded-2xl border border-gray-800 bg-gray-950/80 p-4 shadow-sm">
    <div class="mb-3 flex items-center gap-2 shrink-0">
        <h4 class="text-sm font-semibold text-gray-300">Stat Mods</h4>
    </div>
    <div class="space-y-2">{{blocks}}</div>
</article>"##;

    section_tpl.replace("{{blocks}}", &blocks.join(""))
}

pub fn render_rune_tree(tree: Option<&RuneTreePayload>, error: Option<&str>) -> String {
    if let Some(err) = error {
        let err_tpl = r##"<div id="current-rune-tree-container" class="w-full max-w-4xl mx-auto rounded-2xl border border-red-500/40 bg-red-500/10 p-5 text-sm text-red-300">
    <p>{{error}}</p>
</div>"##;
        return err_tpl.replace("{{error}}", &escape_html(err));
    }

    let t = match tree {
        Some(val) => val,
        None => {
            return r##"<div id="current-rune-tree-container" class="w-full max-w-4xl mx-auto rounded-2xl bg-gray-900 p-5 text-sm text-gray-400">
    Could not load rune tree.
</div>"##.to_string();
        }
    };

    let page_name = escape_html(&t.page_name);
    let page_name_raw = &t.page_name;
    let is_editing = t.is_editing;
    let is_saved_page = t.runeset_id.map(|id| id > 0).unwrap_or(false);

    let runeset_id_val = t
        .runeset_id
        .map(|id| id.to_string())
        .unwrap_or_default();
    let page_id_val = t
        .page_id
        .map(|id| id.to_string())
        .unwrap_or_default();
    let primary_style_id_val = t
        .primary_style_id
        .map(|id| id.to_string())
        .unwrap_or_default();
    let sub_style_id_val = t
        .sub_style_id
        .map(|id| id.to_string())
        .unwrap_or_default();

    let current_perks_str = t
        .selected_perk_ids
        .iter()
        .map(|p| p.to_string())
        .collect::<Vec<_>>()
        .join(",");

    let primary_html = render_style_section(
        t.primary.as_ref(),
        "Primary Tree",
        false,
        is_editing,
        is_saved_page,
        &runeset_id_val,
        &page_id_val,
        &primary_style_id_val,
        &sub_style_id_val,
        &current_perks_str,
    );
    let secondary_html = render_style_section(
        t.secondary.as_ref(),
        "Secondary Tree",
        true,
        is_editing,
        is_saved_page,
        &runeset_id_val,
        &page_id_val,
        &primary_style_id_val,
        &sub_style_id_val,
        &current_perks_str,
    );
    let stat_mods_html = render_stat_mods_section(
        &t.stat_mods,
        is_editing,
        is_saved_page,
        &runeset_id_val,
        &page_id_val,
        &primary_style_id_val,
        &sub_style_id_val,
        &current_perks_str,
    );

    let (container_classes, header_html) = if is_editing {
        (
            "w-full max-w-4xl mx-auto rounded-2xl bg-gray-900 p-5 flex flex-col transition-all",
            format!(
                r##"<div class="mb-5 flex items-center justify-between gap-4 border-b border-gray-800 pb-4">
    <div class="flex items-center gap-3 flex-1 min-w-0">
        <span class="inline-block h-2.5 w-2.5 rounded-full bg-purple-500 shadow-sm shadow-purple-500 shrink-0" title="Companion Saved"></span>
        <input
            type="text"
            id="rune-page-name-input"
            name="pageName"
            value="{}"
            class="w-full max-w-sm rounded-xl border border-gray-700 bg-gray-800 px-3.5 py-2 text-base font-bold text-white placeholder:text-gray-500 focus:border-purple-500 focus:outline-none focus:ring-2 focus:ring-purple-500/40"
            placeholder="Rune page name..."
        />
        <span class="inline-flex items-center gap-1.5 rounded-full border border-purple-500/40 bg-purple-500/10 px-2.5 py-1 text-[11px] font-bold uppercase tracking-wider text-purple-400 shadow-sm shrink-0">
            <span class="h-1.5 w-1.5 rounded-full bg-purple-400 animate-pulse"></span>
            Edit Mode
        </span>
    </div>
    <div class="flex items-center gap-3 shrink-0">
        <button
            type="button"
            title="Cancel changes"
            hx-post="command:ui_rune_tree_cancel_edit"
            hx-vals='{{"runesetId": "{}", "pageId": "{}"}}'
            hx-target="#current-rune-tree-container"
            hx-swap="outerHTML"
            class="inline-flex shrink-0 items-center gap-2 rounded-xl border border-gray-700 bg-gray-800 px-4 py-2 text-sm font-semibold text-gray-200 shadow-sm transition-all hover:bg-gray-700 hover:text-white focus:outline-none focus:ring-4 focus:ring-gray-700 active:scale-95 cursor-pointer"
        >
            <svg class="h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"/></svg>
            <span>Cancel</span>
        </button>
        <button
            type="button"
            title="Save changes to database"
            hx-post="command:ui_save_rune_tree"
            hx-include="#rune-page-name-input"
            hx-vals='{{"runesetId": "{}", "pageId": "{}", "primaryStyleId": "{}", "subStyleId": "{}", "selectedPerks": "{}"}}'
            hx-target="#current-rune-tree-container"
            hx-swap="outerHTML"
            class="inline-flex shrink-0 items-center gap-2 rounded-xl bg-purple-600 px-4 py-2 text-sm font-bold text-white shadow-md shadow-purple-950/40 transition-all hover:bg-purple-500 focus:outline-none focus:ring-4 focus:ring-purple-800 active:scale-95 cursor-pointer"
        >
            <svg class="h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7"/></svg>
            <span>Save</span>
        </button>
    </div>
</div>"##,
                escape_html(page_name_raw),
                runeset_id_val,
                page_id_val,
                runeset_id_val,
                page_id_val,
                primary_style_id_val,
                sub_style_id_val,
                current_perks_str
            ),
        )
    } else {
        let action_button_html = if is_saved_page {
            format!(
                r##"<div class="flex items-center gap-2.5">
    <button
        type="button"
        title="Edit saved rune page"
        hx-post="command:ui_rune_tree_edit_mode"
        hx-vals='{{"runesetId": "{}", "pageId": "{}", "pageName": "{}", "primaryStyleId": "{}", "subStyleId": "{}", "currentPerks": "{}"}}'
        hx-target="#current-rune-tree-container"
        hx-swap="outerHTML"
        class="inline-flex shrink-0 items-center gap-1.5 rounded-xl border border-gray-700 bg-gray-800 px-3.5 py-2 text-sm font-semibold text-gray-200 shadow-sm transition-all hover:bg-gray-700 hover:text-white focus:outline-none focus:ring-4 focus:ring-gray-700 active:scale-95 cursor-pointer"
    >
        <svg class="h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M11 5H6a2 2 0 00-2 2v11a2 2 0 002 2h11a2 2 0 002-2v-5m-1.414-9.414a2 2 0 112.828 2.828L11.828 15H9v-2.828l8.586-8.586z"/>
        </svg>
        <span>Edit</span>
    </button>
    <button
        type="button"
        title="Export runes to League Client"
        hx-post="command:ui_export_saved_rune_page"
        hx-vals='{{"runesetId": "{}"}}'
        hx-target="#current-rune-tree-container"
        hx-swap="outerHTML"
        class="inline-flex shrink-0 items-center gap-2 rounded-xl bg-purple-600 px-4 py-2 text-sm font-bold text-white shadow-md shadow-purple-950/40 transition-all hover:bg-purple-500 focus:outline-none focus:ring-4 focus:ring-purple-800 active:scale-95 cursor-pointer"
    >
        <svg class="h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-8l-4-4m0 0L8 8m4-4v12"/>
        </svg>
        <span>Export</span>
    </button>
</div>"##,
                runeset_id_val,
                page_id_val,
                escape_html(page_name_raw),
                primary_style_id_val,
                sub_style_id_val,
                current_perks_str,
                runeset_id_val
            )
        } else {
            format!(
                r##"<button
    type="button"
    title="Import into Companion database"
    hx-post="command:ui_import_client_page_from_tree"
    hx-vals='{{"pageId": "{}"}}'
    hx-target="#current-rune-tree-container"
    hx-swap="outerHTML"
    class="inline-flex shrink-0 items-center gap-2 rounded-xl bg-purple-600 px-4 py-2 text-sm font-semibold text-white shadow-md shadow-purple-950/40 transition-all hover:bg-purple-500 focus:outline-none focus:ring-4 focus:ring-purple-800 active:scale-95 cursor-pointer"
>
    <svg class="h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-4l-4 4m0 0l-4-4m4 4V4"/>
    </svg>
    <span>Import</span>
</button>"##,
                page_id_val
            )
        };

        let dot_indicator_html = if is_saved_page {
            r##"<span class="inline-block h-2.5 w-2.5 rounded-full bg-purple-500 shadow-sm shadow-purple-500 shrink-0" title="Companion Saved"></span>"##
        } else {
            r##"<span class="inline-block h-2.5 w-2.5 rounded-full bg-blue-500 shadow-sm shadow-blue-500 shrink-0" title="League Client"></span>"##
        };

        (
            "w-full max-w-4xl mx-auto rounded-2xl bg-gray-900 p-5 flex flex-col transition-all",
            format!(
                r##"<div class="mb-5 flex items-center justify-between gap-4 border-b border-gray-800 pb-4">
    <div class="flex items-center gap-3">
        {}
        <h3 class="text-lg font-bold text-white truncate">{}</h3>
    </div>
    <div>
        {}
    </div>
</div>"##,
                dot_indicator_html,
                page_name,
                action_button_html
            ),
        )
    };

    let tree_tpl = r##"<div id="current-rune-tree-container" class="{{container_classes}}">
    {{header_html}}
    <div class="flex flex-row gap-5 items-stretch flex-1">
        <div class="flex-1 min-w-0 flex flex-col">
            {{primary_html}}
        </div>
        <div class="flex-1 min-w-0 flex flex-col gap-4">
            {{secondary_html}}
            {{stat_mods_html}}
        </div>
    </div>
</div>"##;

    tree_tpl
        .replace("{{container_classes}}", container_classes)
        .replace("{{header_html}}", &header_html)
        .replace("{{primary_html}}", &primary_html)
        .replace("{{secondary_html}}", &secondary_html)
        .replace("{{stat_mods_html}}", &stat_mods_html)
}
