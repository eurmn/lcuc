import { invoke } from "@tauri-apps/api/core";

export type LogLevel = "debug" | "info" | "warn" | "error";

function timestamp() {
  return new Date().toISOString();
}

function emit(level: LogLevel, source: string, message: string, context?: unknown) {
  const line = `[${timestamp()}] [${level.toUpperCase()}] [${source}] ${message}`;
  const payload = context === undefined ? undefined : context;
  if (level === "error") console.error(line, payload ?? "");
  else if (level === "warn") console.warn(line, payload ?? "");
  else if (level === "debug") {
    const viteMeta = import.meta as ImportMeta & { env?: { DEV?: boolean } };
    if (viteMeta.env?.DEV === true) console.debug(line, payload ?? "");
  } else console.info(line, payload ?? "");

  void invoke("api_log", {
    level,
    source,
    message,
    context: context === undefined ? null : safeContext(context),
  }).catch(() => {
    // Logging must never break the application if Tauri is unavailable.
  });
}

function safeContext(value: unknown): string {
  try {
    return typeof value === "string" ? value : JSON.stringify(value);
  } catch {
    return "[unserializable context]";
  }
}

export const logger = {
  debug: (source: string, message: string, context?: unknown) => emit("debug", source, message, context),
  info: (source: string, message: string, context?: unknown) => emit("info", source, message, context),
  warn: (source: string, message: string, context?: unknown) => emit("warn", source, message, context),
  error: (source: string, message: string, context?: unknown) => emit("error", source, message, context),
};
