use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuneOption {
    pub id: i64,
    pub name: String,
    pub icon_url: String,
    pub selected: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuneSlot {
    pub label: String,
    pub options: Vec<RuneOption>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuneStyleTree {
    pub id: i64,
    pub name: String,
    pub icon_url: String,
    pub slots: Vec<RuneSlot>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuneTreePayload {
    pub page_name: String,
    pub primary: Option<RuneStyleTree>,
    pub secondary: Option<RuneStyleTree>,
    pub stat_mods: Vec<RuneSlot>,
    #[serde(default)]
    pub runeset_id: Option<i64>,
    #[serde(default)]
    pub page_id: Option<i64>,
    #[serde(default)]
    pub primary_style_id: Option<i64>,
    #[serde(default)]
    pub sub_style_id: Option<i64>,
    #[serde(default)]
    pub selected_perk_ids: Vec<i64>,
    #[serde(default)]
    pub is_editing: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientRunePageSummary {
    pub id: i64,
    pub name: String,
    pub is_active: bool,
    pub is_editable: bool,
    pub primary_style_id: Option<i64>,
    pub sub_style_id: Option<i64>,
    pub primary_style_name: String,
    pub primary_style_icon_url: String,
    pub selected_perk_ids: Vec<i64>,
}
