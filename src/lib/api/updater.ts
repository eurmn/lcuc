import { check, type Update } from "@tauri-apps/plugin-updater";
import { logger } from "../utils/logger";

export interface UpdateInfo {
  version: string;
  currentVersion: string;
  date?: string;
  body?: string;
}

let pendingUpdate: Update | null = null;
let isChecking = false;

// ponytail: basic silent download with restart prompt. Add download progress bar if releases exceed 100MB.
export async function checkForUpdateSilently(): Promise<UpdateInfo | null> {
  if (isChecking || pendingUpdate) {
    if (pendingUpdate) {
      return {
        version: pendingUpdate.version,
        currentVersion: pendingUpdate.currentVersion,
        date: pendingUpdate.date,
        body: pendingUpdate.body,
      };
    }
    return null;
  }

  isChecking = true;
  try {
    const update = await check();
    if (!update) {
      return null;
    }

    logger.info(
      "updater",
      `Update available: v${update.version} (current: v${update.currentVersion}). Downloading in background...`
    );
    await update.download();
    logger.info(
      "updater",
      `Update v${update.version} downloaded and ready to apply on restart.`
    );
    pendingUpdate = update;

    return {
      version: update.version,
      currentVersion: update.currentVersion,
      date: update.date,
      body: update.body,
    };
  } catch (error) {
    logger.warn(
      "updater",
      `Background update check failed: ${error instanceof Error ? error.message : String(error)}`
    );
    return null;
  } finally {
    isChecking = false;
  }
}

export async function applyPendingUpdate(): Promise<void> {
  if (!pendingUpdate) {
    throw new Error("No downloaded update ready to install");
  }

  logger.info(
    "updater",
    `Installing update v${pendingUpdate.version} and restarting application...`
  );
  await pendingUpdate.install({ restartAfterInstall: true });
}

export function getPendingUpdate(): UpdateInfo | null {
  if (!pendingUpdate) return null;
  return {
    version: pendingUpdate.version,
    currentVersion: pendingUpdate.currentVersion,
    date: pendingUpdate.date,
    body: pendingUpdate.body,
  };
}
