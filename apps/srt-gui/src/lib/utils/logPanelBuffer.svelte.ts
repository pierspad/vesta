import type { LogEntry } from "$lib/panels/LogPanel.svelte";

/**
 * Shared reactive log buffer for tabs that render their activity log through
 * `LogPanel.svelte` (its `LogEntry` uses a `type` field). TranscribeTab and
 * TranslateTab used to each redeclare an identical `logs`/`logIdCounter`
 * pair plus a one-line `addLog`/`clearLogs` — this factory replaces both.
 *
 * `generationStore.svelte.ts` (used by FlashcardsTab) is NOT built on this:
 * it coalesces repeated "progress" rows into a single updated entry instead
 * of appending, which is a different enough concern to keep separate. See
 * [[vesta-gui-lib-reorg]].
 */
export function createLogPanelBuffer() {
  let logs = $state<LogEntry[]>([]);
  let logIdCounter = 0;

  function addLog(message: string, type: LogEntry["type"] = "info") {
    const timestamp = new Date().toLocaleTimeString([], {
      hour: "2-digit",
      minute: "2-digit",
      second: "2-digit",
    });
    logs = [...logs, { id: ++logIdCounter, timestamp, message, type }];
  }

  function clearLogs() {
    logs = [];
    logIdCounter = 0;
  }

  return {
    get logs() {
      return logs;
    },
    addLog,
    clearLogs,
  };
}
