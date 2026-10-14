import { invokeCommand } from "./tauriClient";
import { transcribeListModels, transcribeAddonsStatus, transcribeDownloadModel, transcribeDownloadVad } from "./transcribe";
import { downloadFfmpeg } from "./media";
import type { FontStatusInfo } from "$lib/stores/fontStore.svelte";
import type { SetupResources, SetupTask } from "$lib/workflows/firstRunSetup";
interface MediaSetupStatus { ready: boolean; download_bytes: number | null }
export async function loadSetupResources(): Promise<SetupResources> {
  const [models, addons, fonts, media] = await Promise.all([
    transcribeListModels(), transcribeAddonsStatus(), invokeCommand<FontStatusInfo[]>("flashcard_list_fonts"),
    invokeCommand<MediaSetupStatus>("setup_media_status"),
  ]);
  return { models, vadModels: addons.vad_models, fonts, mediaReady: media.ready, mediaDownloadBytes: media.download_bytes };
}
export async function installSetupResource(task: SetupTask): Promise<void> {
  switch (task.kind) {
    case "media": await downloadFfmpeg(); break;
    case "whisper": await transcribeDownloadModel(task.resourceId); break;
    case "vad": await transcribeDownloadVad(task.resourceId); break;
    case "font": {
      const installed = await invokeCommand<boolean>("flashcard_download_font", { fontId: task.resourceId });
      if (!installed) throw new Error(`Font installation failed: ${task.name}`);
      break;
    }
  }
}
