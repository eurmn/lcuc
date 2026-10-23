import type { ChampionSummary } from "./champions";

export interface MatchHistoryItemInfo {
  id: number;
  name: string;
  icon_url: string;
  description?: string;
  price_total?: number;
}

export interface MatchHistoryAugmentInfo {
  id: string;
  name: string;
  icon_url: string;
  rarity?: string;
  tooltip?: string;
}

export interface MatchHistoryItem {
  game_id: number;
  champion: ChampionSummary | null;
  win: boolean;
  game_mode: string;
  game_duration_secs: number;
  game_creation_date: string;
  kills: number;
  deaths: number;
  assists: number;
  champ_level: number;
  minions_killed: number;
  gold_earned: number;
  spell1_id: number;
  spell2_id: number;
  items: Array<MatchHistoryItemInfo | null>;
  damage_dealt_to_champions: number | null;
  augments: MatchHistoryAugmentInfo[];
  map_id: number;
  map_name?: string | null;
  patch?: string | null;
}

export interface BuildStatisticChampion {
  id: number;
  name: string;
  icon_url: string;
  games: number;
  wins: number;
  kills: number;
  deaths: number;
  assists: number;
  damage: number;
  gold: number;
}

export interface ComboComponentDto {
  kind: "champion" | "item" | "augment";
  id: string | number;
  name: string;
  icon_url: string;
  rarity?: string;
  item_class?: string;
  description?: string;
  price_total?: number;
  categories?: string[];
}

export interface BuildStatisticEntry {
  id: string;
  name: string;
  icon_url: string;
  kind: "item" | "augment" | "combo";
  components?: ComboComponentDto[];
  raw_win_rate?: number;
  games: number;
  wins: number;
  kills: number;
  deaths: number;
  assists: number;
  damage: number;
  gold: number;
  duration: number;
  champions: BuildStatisticChampion[];
  partners: Array<{
    id: string;
    name: string;
    icon_url: string;
    games: number;
  }>;
}
