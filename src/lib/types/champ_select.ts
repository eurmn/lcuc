export interface MayhemBuildItem {
  id: number;
  name: string;
  icon_url: string;
  price_total: number;
  item_class?: string;
  description?: string;
  win_rate?: number;
  wpa?: number;
  games?: number;
  pick_rate?: number;
}

export interface MayhemBuildSpell {
  id: number;
  name: string;
  icon_url: string;
  description?: string;
  win_rate?: number;
  wpa?: number;
  games?: number;
}

export interface MayhemBuildAugment {
  id: string;
  name: string;
  icon_url: string;
  rarity?: "silver" | "gold" | "prismatic" | string;
  tooltip?: string;
  win_rate?: number;
  wpa?: number;
  games?: number;
}

export interface MayhemBuildStrategy {
  id: string;
  label: string;
  metric_name: string;
  avg_metric_value: string;
  raw_metric_value: number;
  build_win_rate: number;
  build_wpa: number;
  starter_item?: MayhemBuildItem | null;
  items: MayhemBuildItem[];
  alternative_items?: MayhemBuildItem[];
  situational_items?: MayhemBuildItem[];
  spells: MayhemBuildSpell[];
  all_spells?: MayhemBuildSpell[];
  augments: MayhemBuildAugment[];
  sample_games: number;
}

export interface MayhemChampionBuildSummary {
  champion_id: number;
  champion_name: string;
  icon_url: string;
  champion_win_rate: number;
  champion_wpa: number;
  total_games: number;
  strategies: MayhemBuildStrategy[];
  applied_augment_ids?: string[];
  fallback_used?: boolean;
}

export interface SavedMayhemBuild {
  id: string;
  champion_id: number;
  champion_name: string;
  strategy_label: string;
  augment_ids: string[];
  build: MayhemBuildStrategy;
  created_at: string;
}

export interface SaveMayhemBuildPayload {
  champion_id: number;
  champion_name: string;
  strategy_label: string;
  augment_ids: string[];
  build: MayhemBuildStrategy;
}

export interface MayhemAugmentItem {
  id: string;
  name: string;
  icon_url: string;
  rarity?: "silver" | "gold" | "prismatic" | string;
  tooltip?: string;
  categories: string[];
  games: number;
  wins: number;
  win_rate: number;
  wpa: number;
  pick_rate?: number;
  global_games: number;
  global_win_rate: number;
  global_wpa: number;
  global_pick_rate?: number;
}

export interface MayhemAugmentsQuery {
  champion_id?: number | null;
  tier?: string;
  category?: string;
  search?: string;
  sort_key?: string;
  descending?: boolean;
  page?: number;
  page_size?: number;
  only_sampled?: boolean;
  hide_low_pickrate?: boolean;
  with_augment_ids?: string[];
}

export interface MayhemAugmentsResponse {
  total_items: number;
  total_pages: number;
  page: number;
  page_size: number;
  champion_id?: number | null;
  champion_name?: string | null;
  champion_win_rate: number;
  total_games?: number;
  items: MayhemAugmentItem[];
}

export interface ChampSelectTeamMember {
  cellId: number;
  championId: number;
  assignedPosition?: string;
  spell1Id?: number;
  spell2Id?: number;
  summonerId?: number;
  puuid?: string;
}

export interface ChampSelectBenchChampion {
  championId: number;
  isPriority?: boolean;
}

export interface ChampSelectSession {
  gameId?: number;
  queueId?: number;
  localPlayerCellId: number;
  benchEnabled: boolean;
  benchChampions: ChampSelectBenchChampion[];
  myTeam: ChampSelectTeamMember[];
  timer?: {
    adjustedTimeLeftInPhase?: number;
    phase?: string;
    totalTimeInPhase?: number;
  };
}

export interface ExportBuildPayload {
  champion_id: number;
  champion_name: string;
  strategy_name: string;
  starter_item_id?: number | null;
  item_ids: number[];
  spell1_id: number;
  spell2_id: number;
}

export interface ExportBuildResult {
  success: boolean;
  item_set_created: boolean;
  spells_applied: boolean;
  message: string;
}
