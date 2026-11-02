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
  situational_tag?: string;
}

export interface MayhemStageItem {
  stage: number;
  title: string;
  timing: string;
  items: MayhemBuildItem[];
}

export interface MayhemLiveAdvice {
  current_stage: number;
  stage_title: string;
  immediate_purchases: MayhemBuildItem[];
  next_target?: MayhemBuildItem | null;
  gold_needed: number;
}

export interface LivePlayerItem {
  slot: number;
  item_id: number;
  count: number;
  display_name: string;
  price: number;
}

export interface LivePlayerStats {
  ability_power: number;
  attack_damage: number;
  attack_speed: number;
  attack_range: number;
  crit_chance: number;
  armor: number;
  magic_resist: number;
  move_speed: number;
}

export interface LiveGameState {
  is_in_game: boolean;
  game_time: number;
  champion_name: string;
  level: number;
  current_gold: number;
  items: LivePlayerItem[];
  stats: LivePlayerStats;
  game_mode?: string;
  is_dead?: boolean;
  respawn_timer?: number;
  is_augment_checkpoint?: boolean;
  augment_slot?: number | null;

  // camelCase mirrors
  isInGame?: boolean;
  gameTime?: number;
  championName?: string;
  currentGold?: number;
  gameMode?: string;
  isDead?: boolean;
  respawnTimer?: number;
  isAugmentCheckpoint?: boolean;
  augmentSlot?: number | null;
}

export interface GameWindowRect {
  found: boolean;
  is_foreground: boolean;
  x: number;
  y: number;
  width: number;
  height: number;
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
  pick_rate?: number;
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
  stages?: MayhemStageItem[];
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

export interface OverlaySynergyItem {
  id: number;
  name: string;
  icon_url: string;
  item_type: string;
  total_games: number;
  win_rate: number;
  wpa: number;
}

export interface OverlayComboEntry {
  label: string;
  win_rate: number;
  wpa: number;
  games: number;
  items: OverlaySynergyItem[];
}

export interface OverlayAugmentCard {
  slot: number;
  id: string;
  name: string;
  rarity?: string | null;
  icon_url: string;
  win_rate: number;
  games: number;
  wpa: number;
  tooltip?: string | null;
  synergy_items: OverlaySynergyItem[];
  combos: OverlayComboEntry[];
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

export interface ActiveComboSynergy {
  combo_type: string;
  item_ids: number[];
  augment_ids: string[];
  label: string;
  win_rate: number;
  wpa: number;
  synergy_lift?: number;
  lcb_synergy_lift?: number;
  sample_size: number;
  confidence_grade?: string;

  // camelCase mirrors
  comboType?: string;
  itemIds?: number[];
  augmentIds?: string[];
  winRate?: number;
  synergyLift?: number;
  lcbSynergyLift?: number;
  sampleSize?: number;
  confidenceGrade?: string;
}

export interface ItemRecommendation {
  item_id: number;
  win_rate: number;
  lift: number;
  sample_size: number;
  is_boot: boolean;
  reason: string;
  synergy_lift?: number;
  lcb_lift?: number;
  confidence_grade?: string;

  // camelCase mirrors
  itemId?: number;
  winRate?: number;
  sampleSize?: number;
  isBoot?: boolean;
  synergyLift?: number;
  lcbLift?: number;
  confidenceGrade?: string;
}

export interface EnginePerformanceStats {
  load_time_ms: number;
  compute_time_ms: number;
  total_time_ms: number;
  match_count: number;
  cache_hit: boolean;

  // camelCase mirrors
  loadTimeMs?: number;
  computeTimeMs?: number;
  totalTimeMs?: number;
  matchCount?: number;
  cacheHit?: boolean;
}

export interface AugmentCardRecommendation {
  augment_id: string;
  name: string;
  icon_url: string;
  tier: string;
  tooltip: string;
  score: number;
  wpa: number;
  kit_lift: number;
  lcb_wpa: number;
  sample_size: number;
  verdict: "best_pick" | "reroll" | "neutral" | string;
  confidence_grade: string;
  expected_reroll_gain?: number | null;
  upgrade_probability?: number | null;
  active_combos?: ActiveComboSynergy[];
  lookahead_partner_id?: number | null;
  lookahead_partner_kind?: "item" | "augment" | string | null;
  lookahead_synergy_lift?: number | null;

  // camelCase mirrors
  augmentId?: string;
  iconUrl?: string;
  kitLift?: number;
  lcbWpa?: number;
  sampleSize?: number;
  confidenceGrade?: string;
  expectedRerollGain?: number | null;
  upgradeProbability?: number | null;
  activeCombos?: ActiveComboSynergy[];
  lookaheadPartnerId?: number | null;
  lookaheadPartnerKind?: string | null;
  lookaheadSynergyLift?: number | null;
}

export interface RerollEvRecommendation {
  ev_reroll: number;
  pool_size: number;
  tier: string;
  target_tier: string;
  best_pick_id?: string | null;
  should_reroll: boolean;

  // camelCase mirrors
  evReroll?: number;
  poolSize?: number;
  targetTier?: string;
  bestPickId?: string | null;
  shouldReroll?: boolean;
}

export interface AugmentRecommendation {
  augment_id: string;
  name: string;
  icon_url: string;
  tier: string;
  tooltip: string;
  score: number;
  wpa: number;
  kit_lift: number;
  lcb_wpa: number;
  sample_size: number;
  confidence_grade: string;
  lookahead_partner_id?: number | null;
  lookahead_partner_kind?: string | null;
  lookahead_synergy_lift?: number | null;

  // camelCase mirrors
  augmentId?: string;
  iconUrl?: string;
  kitLift?: number;
  lcbWpa?: number;
  sampleSize?: number;
  confidenceGrade?: string;
  lookaheadPartnerId?: number | null;
  lookaheadPartnerKind?: string | null;
  lookaheadSynergyLift?: number | null;
}

export interface BuildRecommendationResult {
  champion_id: number;
  champion_baseline_winrate?: number;
  build_winrate?: number;
  build_wpa?: number;
  active_combos: ActiveComboSynergy[];
  anti_synergies?: ActiveComboSynergy[];
  evaluated_cards?: AugmentCardRecommendation[];
  reroll_ev?: RerollEvRecommendation | null;
  next_items: ItemRecommendation[];
  recommended_boots: ItemRecommendation | null;
  completed_build: ItemRecommendation[];
  alternative_items: ItemRecommendation[];
  recommended_augments_by_tier?: Record<string, AugmentRecommendation[]>;
  stats: EnginePerformanceStats;

  // camelCase mirrors
  championId?: number;
  championBaselineWinrate?: number;
  buildWinrate?: number;
  buildWpa?: number;
  activeCombos?: ActiveComboSynergy[];
  antiSynergies?: ActiveComboSynergy[];
  evaluatedCards?: AugmentCardRecommendation[];
  rerollEv?: RerollEvRecommendation | null;
  nextItems?: ItemRecommendation[];
  recommendedBoots?: ItemRecommendation | null;
  completedBuild?: ItemRecommendation[];
  alternativeItems?: ItemRecommendation[];
  recommendedAugmentsByTier?: Record<string, AugmentRecommendation[]>;
}

export interface MayhemComboDto {
  id: string;
  name: string;
  category: string;
  champion_id?: number;
  championId?: number;
  components: Array<{
    kind: string;
    id: string | number;
    name: string;
    icon_url: string;
    rarity?: string;
    item_class?: string;
    description?: string;
    price_total?: number;
    categories?: string[];
  }>;
  games: number;
  wins: number;
  win_rate: number;
  winRate?: number;
  combo_wpa: number;
  comboWpa?: number;
  wpa: number;
  synergy_lift: number;
  synergyLift?: number;
  lift?: number;
  synergy_score?: number;
  synergyScore?: number;
  lcb_synergy_lift: number;
  lcbSynergyLift?: number;
  confidence_grade: string;
  confidenceGrade?: string;
}

