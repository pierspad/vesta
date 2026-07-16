import { t } from "./i18n";

export interface AutoSyncProgressPayload {
  stage: string;
  message: string;
  percentage: number;
  message_key?: string | null;
  params?: Record<string, string> | null;
}

class AutoSyncStore {
  isAutoSyncing = $state(false);
  activeMode = $state<"quick" | "precise" | null>(null);
  selectedMode = $state<"quick" | "precise">("quick");
  isCancelling = $state(false);
  progress = $state(0);
  message = $state("");
  whisperModelsAvailable = $state<string[]>([]);

  resolveProgressMessage(payload: AutoSyncProgressPayload): string {
    if (payload.message_key) {
      const params = payload.params ?? {};

      // Keep compatibility with locale strings that expect {{count}} while
      // backend progress events provide { total } for segment counts.
      if (params.total && !params.count) {
        return t(payload.message_key, { ...params, count: params.total });
      }

      return t(payload.message_key, params);
    }
    return payload.message;
  }

  formatModeName(key: string, fallback: string): string {
    const value = t(key);
    if (value.includes(" - ")) {
      const part = value.split(" - ")[1];
      if (part) {
        return part.charAt(0).toUpperCase() + part.slice(1).toLowerCase();
      }
    }
    return fallback;
  }

  toggleMode() {
    if (!this.isAutoSyncing) {
      this.selectedMode = this.selectedMode === "quick" ? "precise" : "quick";
    }
  }
}

export const autoSyncStore = new AutoSyncStore();
