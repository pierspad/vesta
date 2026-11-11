// This store deliberately uses raw IPC: recording its own writes would recurse.
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { setSupportLogSink, recordSupportEvent, safeLogValue, type SupportEntry } from "$lib/utils/supportLog";

export class SupportLogStore {
  recording = $state(false);
  path = $state<string | null>(null);
  busy = $state(false);
  error = $state("");
  preview = $state<SupportEntry[]>([]);
  previewText = $derived(this.preview.map(entry => `[${entry.time}] ${entry.kind}: ${entry.message}`).join("\n"));
  private entries: SupportEntry[] = [];
  private recentEntries: SupportEntry[] = [];
  private previewScheduled = false;
  private acceptingEntries = false;
  private timer: ReturnType<typeof setInterval> | null = null;
  private pending: Promise<void> | null = null;
  private initialization: Promise<void> | null = null;

  constructor() {
    setSupportLogSink(entry => {
      if (this.acceptingEntries) this.entries.push(entry);
      this.recentEntries.push(entry);
      if (this.recentEntries.length > 200) this.recentEntries.shift();
      if (this.previewScheduled) return;

      // Commands can log inside Svelte effects. Publish outside their reactive context.
      this.previewScheduled = true;
      queueMicrotask(() => {
        this.previewScheduled = false;
        this.preview = this.recentEntries.slice();
      });
    });
  }

  initialize(): Promise<void> {
    return this.initialization ??= this.connect();
  }

  private async connect() {
    const events = ["flashcard-progress", "translate-progress", "translate-complete", "transcribe-progress", "transcribe-complete", "refine-progress", "sync-auto-progress", "sync-auto-complete", "provider-retry", "plugin:notify", "plugin:progress", "font-download-progress", "update-download-progress"];
    await Promise.all(events.map(async name => {
      try {
        await listen<Record<string, unknown> | number>(name, event => {
          if (typeof event.payload === "number") {
            recordSupportEvent(name, safeLogValue(event.payload));
            return;
          }
          if (name === "plugin:notify" || name === "plugin:progress") {
            const { pluginId, level, value } = event.payload || {};
            recordSupportEvent(name, safeLogValue({ pluginId, level, value }));
            return;
          }
          if (name === "provider-retry") {
            const { tier, endpoint, status, attempt, max_retries, retry_in_ms } = event.payload || {};
            recordSupportEvent(name, safeLogValue({ tier, endpoint, status, attempt, max_retries, retry_in_ms }));
            return;
          }
          const { message, percentage, current_batch, total_batches, success, cancelled, stage, current, total, percent, error } = event.payload || {};
          recordSupportEvent(name, safeLogValue({ message, percentage, current_batch, total_batches, success, cancelled, stage, current, total, percent, error }));
        });
      } catch (error) { this.error = safeLogValue(error); }
    }));
    try {
      const status = await invoke<{ path: string | null; recording: boolean }>("support_log_status");
      this.path = status.path;
      if (status.recording) this.startWriter();
    } catch (error) { this.error = safeLogValue(error); }
  }

  private startWriter() {
    this.acceptingEntries = true;
    this.recording = true;
    this.timer = setInterval(() => {
      if (!this.busy && !this.pending) {
        void this.flush().catch(error => { this.error = safeLogValue(error); });
      }
    }, 500);
  }
  async start() {
    if (this.busy || this.recording) return;
    this.busy = true; this.error = "";
    try {
      await this.initialize();
      if (this.recording) return;
      this.path = await invoke<string>("support_log_start");
      this.startWriter();
      recordSupportEvent("session", "Recording enabled");
    } catch (error) { this.error = safeLogValue(error); }
    finally { this.busy = false; }
  }
  private async writeQueuedEntries() {
    // Drain the current snapshot so a busy workflow cannot delay export indefinitely.
    let remaining = this.entries.length;
    while (remaining > 0) {
      const entries = this.entries.splice(0, Math.min(remaining, 100));
      try {
        await invoke<void>("support_log_append", { path: this.path, entries: entries.map(entry => JSON.stringify(entry)) });
      } catch (error) {
        this.entries.unshift(...entries);
        throw error;
      }
      remaining -= entries.length;
    }
  }

  private flush(): Promise<void> {
    this.pending = this.writeQueuedEntries().finally(() => { this.pending = null; });
    return this.pending;
  }

  async stop() {
    if (this.busy || !this.recording) return;
    this.busy = true; this.error = "";
    recordSupportEvent("session", "Recording stopped");
    this.acceptingEntries = false;
    this.recording = false;
    if (this.timer) clearInterval(this.timer);
    this.timer = null;
    try {
      if (this.pending) await this.pending;
      await this.flush();
      await invoke("support_log_stop");
    } catch (error) {
      this.error = safeLogValue(error);
      this.startWriter();
    } finally { this.busy = false; }
  }

  async export(destination: string) {
    if (this.busy || !this.path) return null;
    this.busy = true; this.error = "";
    try {
      if (this.pending) await this.pending;
      await this.flush();
      return await invoke<string>("support_log_export", { destination });
    } catch (error) { this.error = safeLogValue(error); return null; }
    finally { this.busy = false; }
  }
}
export const supportLogStore = new SupportLogStore();
