import htmx from "htmx.org";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import "./styles.css";

declare global {
  interface Window {
    htmx: typeof htmx;
  }
}

window.htmx = htmx;
htmx.config.selfRequestsOnly = false;

function setupTitlebar(): void {
  try {
    const appWindow = getCurrentWindow();

    const minBtn = document.getElementById("titlebar-minimize");
    const maxBtn = document.getElementById("titlebar-maximize");
    const closeBtn = document.getElementById("titlebar-close");

    minBtn?.addEventListener("click", () => {
      appWindow.minimize();
    });

    maxBtn?.addEventListener("click", () => {
      appWindow.toggleMaximize();
    });

    closeBtn?.addEventListener("click", () => {
      appWindow.close();
    });
  } catch (err) {
    console.error("[Titlebar] Failed to initialize window controls:", err);
  }
}

const COMMAND_PREFIX = "command:";

function cleanArgValue(value: any): any {
  if (value === null || value === undefined || value === "null" || value === "undefined") {
    return null;
  }
  if (typeof value === "string") {
    const trimmed = value.trim();
    if (trimmed === "" || trimmed === "null" || trimmed === "undefined") {
      return null;
    }
    const num = Number(trimmed);
    return !isNaN(num) ? num : trimmed;
  }
  return value;
}

document.addEventListener("htmx:beforeSend", (event: any) => {
  const requestConfig = event.detail.requestConfig;
  const rawPath = requestConfig.path;

  if (rawPath && rawPath.startsWith(COMMAND_PREFIX)) {
    const fullCommandStr = rawPath.slice(COMMAND_PREFIX.length);
    const [commandName, queryString] = fullCommandStr.split("?", 2);

    const args: Record<string, any> = {};

    if (queryString) {
      const searchParams = new URLSearchParams(queryString);
      searchParams.forEach((value, key) => {
        args[key] = cleanArgValue(value);
      });
    }

    if (requestConfig.parameters) {
      for (const [key, value] of Object.entries(requestConfig.parameters)) {
        args[key] = cleanArgValue(value);
      }
    }

    const xhr = event.detail.xhr;
    xhr.command = commandName;
    xhr.commandArgs = args;

    xhr.send = async function (bodyParams: any) {
      Object.defineProperty(this, "readyState", { writable: true, value: XMLHttpRequest.LOADING });
      Object.defineProperty(this, "status", { writable: true });
      Object.defineProperty(this, "statusText", { writable: true });
      Object.defineProperty(this, "response", { writable: true });

      const finalArgs = { ...this.commandArgs };
      if (bodyParams && typeof bodyParams === "string") {
        try {
          const parsed = JSON.parse(bodyParams);
          for (const [key, value] of Object.entries(parsed)) {
            finalArgs[key] = cleanArgValue(value);
          }
        } catch {
          const searchParams = new URLSearchParams(bodyParams);
          searchParams.forEach((value, key) => {
            finalArgs[key] = cleanArgValue(value);
          });
        }
      }

      // Filter out null or undefined if needed, or pass clean nulls
      const payload: Record<string, any> = {};
      for (const [key, value] of Object.entries(finalArgs)) {
        if (value !== null && value !== undefined) {
          payload[key] = value;
        }
      }

      try {
        const result = await invoke(this.command, payload);
        this.response = typeof result === "string" ? result : JSON.stringify(result);
        this.readyState = XMLHttpRequest.DONE;
        this.status = 200;
        this.statusText = "OK";
      } catch (err: any) {
        console.error(`[Tauri HTMX Bridge] Error running ${this.command}:`, err);
        const errMsg = typeof err === "string" ? err : JSON.stringify(err);
        this.response = `<div class="rounded-xl border border-red-500/30 bg-red-500/10 p-3 text-xs text-red-300">Command error (${this.command}): ${errMsg}</div>`;
        this.readyState = XMLHttpRequest.DONE;
        this.status = 200;
        this.statusText = "Error";
      }

      this.dispatchEvent(new ProgressEvent("load"));
      this.dispatchEvent(new ProgressEvent("loadend"));
    };
  }
});

function init(): void {
  setupTitlebar();
  htmx.process(document.body);
}

if (document.readyState === "loading") {
  document.addEventListener("DOMContentLoaded", init);
} else {
  init();
}
