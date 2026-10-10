import { tauriInvoke } from './client';
import type { MatchHistoryItem } from '../types/history';

export async function getMatchHistory(
  begIndex = 0,
  endIndex = 20
): Promise<MatchHistoryItem[]> {
  return tauriInvoke<MatchHistoryItem[]>('api_match_history', {
    beg_index: begIndex,
    end_index: endIndex
  });
}
