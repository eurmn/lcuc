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
