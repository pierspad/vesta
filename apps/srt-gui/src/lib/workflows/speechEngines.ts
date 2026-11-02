import type { TranscribeTier, TranscribeTierEntry } from "$lib/config/transcribeTiers";
import type { ApiKeyConfig } from "$lib/config/apiKeys";
import type { WhisperModel } from "$lib/services/transcribe";
import { isLocalTranscribeProvider, isTranscribeEndpointReady } from "$lib/workflows/transcription";

/** Custom endpoints are validated for real timestamps by the native backend. */
export function supportsSyncTimestamps(entry: TranscribeTierEntry): boolean {
  return !(entry.provider === "openai" && entry.model.startsWith("gpt-4o-"));
}

export function availableSpeechEngines(tiers: TranscribeTier[], models: WhisperModel[], keys: ApiKeyConfig[], sync: boolean): TranscribeTierEntry[] {
  const local = models.filter(model => model.downloaded).map(model => ({
    id: `local:${model.id}`, provider: "local", model: model.id, apiKeyId: "local_whisper",
  }));
  const external = tiers.flatMap(tier => tier.entries).filter(entry =>
    !isLocalTranscribeProvider(entry.provider) && isTranscribeEndpointReady(entry, models, keys)
      && (!sync || supportsSyncTimestamps(entry))
  );
  return [...local, ...external.filter((entry, index) => external.findIndex(other => other.id === entry.id) === index)];
}
