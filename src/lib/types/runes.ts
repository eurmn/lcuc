export interface RuneOption {
  id: number;
  name: string;
  icon_url: string;
  selected: boolean;
}

export interface RuneSlot {
  label: string;
  options: RuneOption[];
}

export interface RuneStyleTree {
  id: number;
  name: string;
  icon_url: string;
  slots: RuneSlot[];
}

export interface RuneTreePayload {
  page_name: string;
  primary: RuneStyleTree | null;
  secondary: RuneStyleTree | null;
  stat_mods: RuneSlot[];
  runeset_id?: number | null;
  page_id?: number | null;
  primary_style_id?: number | null;
  sub_style_id?: number | null;
  selected_perk_ids: number[];
  champion_id?: number | null;
  is_editing: boolean;
}

export interface ClientRunePageSummary {
  id: number;
  name: string;
  is_active: boolean;
  is_editable: boolean;
  primary_style_id: number | null;
  sub_style_id: number | null;
  primary_style_name: string;
  primary_style_icon_url: string;
  selected_perk_ids: number[];
}

export interface RuneSetSummary {
  id: number;
  rune_page_name: string;
  primary_style_id: number | null;
  sub_style_id: number | null;
  primary_style_name: string;
  primary_style_icon_url: string;
  champion_id: number;
  source: string;
  created_at: string;
}

export interface RunesOverviewPayload {
  client_pages: ClientRunePageSummary[];
  saved_runesets: RuneSetSummary[];
  active_companion_name: string | null;
}

export interface SelectPerkPayload {
  runeset_id?: number | null;
  page_id?: number | null;
  page_name?: string | null;
  primary_style_id?: number | null;
  sub_style_id?: number | null;
  current_perks: number[];
  clicked_perk_id: number;
  slot_type: "primary" | "secondary" | "stat";
  slot_index: number;
}

export interface SaveRuneTreePayload {
  runeset_id?: number | null;
  page_id?: number | null;
  page_name: string;
  primary_style_id?: number | null;
  sub_style_id?: number | null;
  selected_perks: number[];
  champion_id?: number | null;
}
