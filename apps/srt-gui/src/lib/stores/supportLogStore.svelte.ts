// This store deliberately uses raw IPC: recording its own writes would recurse.
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { setSupportLogSink, recordSupportEvent, safeLogValue, type SupportEntry } from "$lib/utils/supportLog";

class SupportLogStore {
  recording = $state(false);
  path = $state<string | null>(null);
  busy = $state(false);
  error = $state("");
  private entries: SupportEntry[] = [];
  private droppedEntries = 0;
  private timer: ReturnType<typeof setInterval> | null = null;
  private pending: Promise<void> | null = null;
  private unlisteners: UnlistenFn[] = [];

  async initialize() {
    try {
      const status = await invoke<{ path: string | null; recording: boolean }>("support_log_status");
      this.path = status.path;
      if (status.recording && !this.recording) await this.attach();
    } catch (error) { this.error = safeLogValue(error); }
  }
  private async attach() {
    this.recording = true;
    setSupportLogSink(entry => {
      if (this.entries.length < 1000) this.entries.push(entry);
      else this.droppedEntries += 1;
    });
    this.timer = setInterval(() => { if (!this.busy) void this.flush().catch(error => { this.error = safeLogValue(error); }); }, 500);
    // Progress messages include native processing failures and workflow milestones.
    for (const name of ["flashcard-progress", "translate-progress", "translate-complete", "transcribe-progress", "transcribe-complete", "refine-progress", "sync-auto-progress", "sync-auto-complete"]) {
      try {
        this.unlisteners.push(await listen<Record<string, unknown>>(name, event => {
          const { message, percentage, current_batch, total_batches, success } = event.payload || {};
          recordSupportEvent(name, safeLogValue({ message, percentage, current_batch, total_batches, success }));
        }));
      } catch (error) { this.error = safeLogValue(error); }
    }
  }
  async start() {
    if (this.busy || this.recording) return;
    this.busy = true; this.error = "";
    try {
      this.path = await invoke<string>("support_log_start");
      await this.attach();
      recordSupportEvent("session", "Recording enabled");
    } catch (error) { this.error = safeLogValue(error); }
    finally { this.busy = false; }
  }
  private flush(): Promise<void> {
    if (this.pending) return this.pending.then(() => this.flush());
    if (!this.entries.length || !this.path) return Promise.resolve();
    if (this.droppedEntries && this.entries.length < 1000) {
      this.entries.push({ time: new Date().toISOString(), kind: "logger.warning", message: `${this.droppedEntries} entries dropped while the log writer was unavailable or overloaded` });
      this.droppedEntries = 0;
    }
    const entries = this.entries.splice(0, 100);
    this.pending = invoke<void>("support_log_append", { path: this.path, entries: entries.map(entry => JSON.stringify(entry)) })
      .catch(error => { this.entries.unshift(...entries); throw error; })
      .finally(() => { this.pending = null; });
    return this.pending;
  }
  async stop() {
    if (this.busy) return;
    this.busy = true; this.error = "";
    recordSupportEvent("session", "Recording stopped");
    setSupportLogSink(null);
    if (this.timer) clearInterval(this.timer);
    this.timer = null;
    for (const unlisten of this.unlisteners) unlisten();
    this.unlisteners = [];
    try {
      while (this.entries.length || this.pending) await this.flush();
      await invoke("support_log_stop");
      this.recording = false;
    } catch (error) {
      this.error = safeLogValue(error);
      // Restore recording if the file cannot be flushed: never silently discard logs.
      await this.attach();
    } finally { this.busy = false; }
  }
  async export(destination: string) {
    this.busy = true; this.error = "";
    try {
      if (this.pending) await this.pending;
      const snapshot = this.entries.splice(0);
      for (let index = 0; index < snapshot.length; index += 100) {
        const entries = snapshot.slice(index, index + 100);
        try {
          await invoke("support_log_append", { path: this.path, entries: entries.map(entry => JSON.stringify(entry)) });
        } catch (error) { this.entries.unshift(...snapshot.slice(index)); throw error; }
      }
      return await invoke<string>("support_log_export", { destination });
    } catch (error) { this.error = safeLogValue(error); return null; }
    finally { this.busy = false; }
  }
}
export const supportLogStore = new SupportLogStore();
