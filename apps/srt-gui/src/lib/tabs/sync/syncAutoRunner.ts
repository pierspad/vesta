import { invokeCommand as invoke } from "$lib/services/tauriClient";
import { localWhisperOperation } from "$lib/utils/asyncControl.svelte";
import { autoSyncStore, type AutoSyncProgressPayload } from "$lib/stores/autoSyncStore.svelte";
import type { TranscribeTierEntry } from "$lib/config/transcribeTiers";
import { loadAndValidateApiKeys } from "$lib/config/apiKeys";
import { transcribeProviders } from "$lib/config/transcribeProviders";
import { isLocalTranscribeProvider } from "$lib/workflows/transcription";
import { errorMessage } from "$lib/utils/errorMessage";
import type { SyncStatus } from "./syncFormatters";

interface ExecuteAutoSyncParams {
  quick: boolean;
  entry: TranscribeTierEntry | null | undefined;
  checkWhisperModels: () => Promise<void>;
  onStatus: (status: SyncStatus) => void;
  refreshAfterSync: () => Promise<void>;
  showSnackbar: (message: string, level: "success" | "info" | "warning" | "error") => void;
  t: (key: string, params?: Record<string, string | number>) => string;
}

export async function executeAutoSync({
  quick,
  entry,
  checkWhisperModels,
  onStatus,
  refreshAfterSync,
  showSnackbar,
  t,
}: ExecuteAutoSyncParams): Promise<void> {
  const resource = localWhisperOperation.begin("autosync");
  if (!resource) {
    showSnackbar(t("common.error.alreadyRunning"), "warning");
    return;
  }
  const run = autoSyncStore.operation.begin();
  if (!run) {
    resource.finish();
    return;
  }

  let unlisten: (() => void) | undefined;
  try {
    if (!entry) {
      showSnackbar(t("transcribe.blocked.noTiers"), "warning");
      return;
    }
    const local = isLocalTranscribeProvider(entry.provider);
    if (local) {
      await checkWhisperModels();
      if (run.isCancelled()) return;
      if (!autoSyncStore.whisperModelsAvailable.includes(entry.model)) {
        showSnackbar(t("transcribe.blocked.missingModel"), "warning");
        return;
      }
    }

    const key = local ? undefined : loadAndValidateApiKeys().find((k) => k.id === entry.apiKeyId);
    const cloud = local
      ? null
      : {
          provider: entry.provider,
          model: entry.model,
          api_key: key?.apiKey?.trim() || "",
          api_url: key?.apiUrl?.trim() || transcribeProviders[entry.provider]?.defaultUrl || null,
          language: null,
          translate_to_english: false,
        };

    autoSyncStore.activeMode = quick ? "quick" : "precise";
    autoSyncStore.progress = 0;
    autoSyncStore.message = t("sync.autoSyncInProgress");

    const { listen } = await import("@tauri-apps/api/event");
    unlisten = await listen<AutoSyncProgressPayload>("sync-auto-progress", (event) => {
      if (!run.isCurrent() || run.isCancelled()) return;
      autoSyncStore.progress = event.payload.percentage;
      autoSyncStore.message = autoSyncStore.resolveProgressMessage(event.payload);
    });

    if (run.isCancelled()) return;
    const result = await invoke<{
      success: boolean;
      cancelled: boolean;
      anchors_created: number;
      segments_analyzed: number;
      message: string;
    }>("sync_auto_sync", {
      modelId: entry.model,
      language: null,
      quick,
      cloud,
    });

    onStatus(await invoke<SyncStatus>("sync_get_status"));
    await refreshAfterSync();

    let summaryMessage = "";
    let summaryLevel: "success" | "info" | "warning" | "error" = "warning";
    if (result.cancelled || run.isCancelled()) {
      summaryLevel = "info";
      summaryMessage = t("sync.autoSyncResult.cancelled");
    } else if (result.success) {
      if (result.anchors_created > 0) {
        summaryMessage = t("sync.autoSyncResult.success", { count: result.anchors_created });
        summaryLevel = "success";
      } else {
        summaryMessage = result.message;
        summaryLevel = "info";
      }
    } else {
      summaryMessage = t("sync.autoSyncResult.noMatches");
    }
    showSnackbar(summaryMessage, summaryLevel);
  } catch (e) {
    if (run.isCancelled()) {
      showSnackbar(t("sync.autoSyncResult.cancelled"), "info");
      return;
    }
    showSnackbar(`Auto-sync failed: ${errorMessage(e)}`, "error");
  } finally {
    run.finish();
    resource.finish();
    autoSyncStore.activeMode = null;
    autoSyncStore.progress = 0;
    autoSyncStore.message = "";
    unlisten?.();
  }
}
