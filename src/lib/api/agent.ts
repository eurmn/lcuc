import { invoke } from '@tauri-apps/api/core';

export interface AgentStatus {
  running: boolean;
  reachable: boolean;
  port: number | null;
  collector_state: string | null;
  database_ready: boolean;
  error: string | null;
}

export function getAgentStatus(): Promise<AgentStatus> {
  return invoke<AgentStatus>('api_agent_status');
}

export interface AgentBuildStatistics {
  facts: Array<Record<string, unknown>>;
}

export function getAgentBuildStatistics(): Promise<AgentBuildStatistics> {
  return invoke<AgentBuildStatistics>('api_build_statistics');
}

export type StatisticsScope = 'personal' | 'compare' | 'global';
export type StatisticsKind = 'champion' | 'item' | 'augment' | 'combo';

export interface ServerStatisticContext {
  id: string | number;
  name: string;
  icon_url: string;
  games: number;
  wins: number;
  win_rate: number;
  wpa: number;
  pick_rate: number;
  rarity?: string;
  item_class?: string;
  description?: string;
  price_total?: number;
}

export interface ComboComponentDto {
  kind: 'champion' | 'item' | 'augment';
  id: string | number;
  name: string;
  icon_url: string;
  rarity?: string;
  item_class?: string;
  description?: string;
  price_total?: number;
  categories?: string[];
}

export interface ServerStatisticRow {
  id: string;
  name: string;
  icon_url: string;
  kind: StatisticsKind;
  rarity?: string;
  item_class?: string;
  description?: string;
  price_total?: number;
  categories?: string[];
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
  win_rate: number;
  wpa: number;
  pick_rate: number;
  damage_per_minute: number;
  champions: ServerStatisticContext[];
  partners: ServerStatisticContext[];
  personal_games?: number;
  personal_win_rate?: number;
  personal_wpa?: number;
  global_games?: number;
  global_win_rate?: number;
  global_wpa?: number;
  personal_wpa_vs_global_win_rate?: number;
  global_observed?: boolean;
}

export interface ServerStatisticsResponse {
  scope: StatisticsScope;
  kind: StatisticsKind;
  sample_count: number;
  baseline: number;
  personal_sample_count?: number;
  personal_baseline?: number;
  global_sample_count?: number;
  global_baseline?: number;
  total: number;
  offset: number;
  limit: number;
  rows: ServerStatisticRow[];
}

export interface ServerStatisticsQuery {
  scope: StatisticsScope;
  kind: StatisticsKind;
  search?: string;
  hide_low_pickrate?: boolean;
  sort_key?: string;
  descending?: boolean;
  limit?: number;
  offset?: number;
}

export function getStatistics(query: ServerStatisticsQuery): Promise<ServerStatisticsResponse> {
  return invoke<ServerStatisticsResponse>('api_statistics', { query });
}

export interface GlobalAramPatchBreakdown {
  patch: string;
  games: number;
}

export interface GlobalAramConfig {
  limit: number;
  games: number;
  workers?: number;
  database_path: string;
  collector_state: string;
  queued_jobs: number;
  current_patch?: string | null;
  patches?: GlobalAramPatchBreakdown[];
  outdated_games?: number;
  has_patch_mismatch?: boolean;
  operations: AgentOperation[];
}

export interface AgentOperation {
  id: string;
  message: string;
  current: number | null;
  total: number | null;
}

export function getGlobalAramConfig(): Promise<GlobalAramConfig> {
  return invoke<GlobalAramConfig>('api_global_aram_config');
}

export function setGlobalAramLimit(limit: number): Promise<{ limit: number }> {
  return invoke<{ limit: number }>('api_set_global_aram_limit', { limit });
}

export function setGlobalAramWorkers(workers: number): Promise<{ workers: number }> {
  return invoke<{ workers: number }>('api_set_global_aram_workers', { workers });
}

export function resetGlobalAramDatabase(): Promise<{
  reset: boolean;
  limit: number;
  games: number;
  workers: number;
  queued_jobs: number;
}> {
  return invoke('api_reset_global_aram');
}

export function getGlobalAramStatistics(): Promise<AgentBuildStatistics> {
  return invoke<AgentBuildStatistics>('api_global_aram_statistics');
}
