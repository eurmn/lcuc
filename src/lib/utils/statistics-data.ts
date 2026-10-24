import { getStatistics, type ServerStatisticsQuery, type ServerStatisticsResponse } from "../api/agent";
import type { StatisticsKind, StatisticsScope } from "../api/agent";
import { logger } from "./logger";

export type { StatisticsKind, StatisticsScope };

const cache = new Map<string, ServerStatisticsResponse>();
const requests = new Map<string, Promise<ServerStatisticsResponse>>();
const PATCH_STORAGE_KEY = "lcu-cached-patch";

export function clearStatisticsCache() {
  cache.clear();
  try {
    const keysToRemove: string[] = [];
    for (let i = 0; i < localStorage.length; i++) {
      const k = localStorage.key(i);
      if (k && k.startsWith("lcu-server-statistics:")) {
        keysToRemove.push(k);
      }
    }
    for (const k of keysToRemove) {
      localStorage.removeItem(k);
    }
  } catch {
    // Ignore storage access errors
  }
}

export function syncStatisticsCachePatch(patch: string | null | undefined): boolean {
  if (!patch) return false;
  try {
    const previous = localStorage.getItem(PATCH_STORAGE_KEY);
    if (previous && previous !== patch) {
      logger.info(
        "frontend.statistics",
        `Detected patch update (${previous} -> ${patch}); invalidating frontend caches`,
      );
      clearStatisticsCache();
      localStorage.removeItem("lcu-match-history");
      localStorage.setItem(PATCH_STORAGE_KEY, patch);
      return true;
    }
    if (!previous) {
      localStorage.setItem(PATCH_STORAGE_KEY, patch);
    }
  } catch {
    // Ignore storage errors
  }
  return false;
}

function isPoroSnax(row: ServerStatisticsResponse["rows"][number]) {
  return row.name.toLowerCase().includes("poro-snax") || row.id === "item:2050" || row.id === "item:2052";
}

function key(query: ServerStatisticsQuery) {
  return JSON.stringify({
    version: 6,
    scope: query.scope,
    kind: query.kind,
    search: query.search ?? "",
    hide_low_pickrate: query.hide_low_pickrate ?? false,
    sort_key: query.sort_key ?? "wpa",
    descending: query.descending ?? true,
    limit: query.limit ?? 100,
    offset: query.offset ?? 0,
  });
}

export function getCachedStatistics(query: ServerStatisticsQuery) {
  const requestKey = key(query);
  const memory = cache.get(requestKey);
  if (memory) return memory;
  try {
    const persisted = localStorage.getItem(`lcu-server-statistics:${requestKey}`);
    if (!persisted) return null;
    const response = JSON.parse(persisted) as ServerStatisticsResponse;
    const cleaned = {
      ...response,
      rows: response.rows.filter((row) => !isPoroSnax(row)),
    };
    cache.set(requestKey, cleaned);
    return cleaned;
  } catch {
    return null;
  }
}

export async function requestStatistics(
  query: ServerStatisticsQuery,
  options: { force?: boolean } = {},
): Promise<ServerStatisticsResponse> {
  const requestKey = key(query);
  if (!options.force) {
    const cached = cache.get(requestKey);
    if (cached) return cached;
    const pending = requests.get(requestKey);
    if (pending) return pending;
  }

  const request = getStatistics(query)
    .then((response) => {
      const cleaned = {
        ...response,
        rows: response.rows.filter((row) => !isPoroSnax(row)),
      };
      cache.set(requestKey, cleaned);
      try {
        localStorage.setItem(`lcu-server-statistics:${requestKey}`, JSON.stringify(cleaned));
      } catch {
        // Browser storage is an optimization; the in-memory response remains valid.
      }
      return cleaned;
    })
    .catch((error) => {
      logger.warn("frontend.statistics", "Server statistics request failed", error);
      throw error;
    })
    .finally(() => requests.delete(requestKey));
  requests.set(requestKey, request);
  return request;
}
