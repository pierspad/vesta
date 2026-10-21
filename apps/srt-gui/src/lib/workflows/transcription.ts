import type { TranscribeConfig, TranscribeStartResult, WhisperModel } from "$lib/services/transcribe";
import type { TranscribeTier, TranscribeTierEntry } from "$lib/config/transcribeTiers";
import { supportsCapability, type ApiKeyConfig } from "$lib/config/apiKeys";
import { transcribeProviders } from "$lib/config/transcribeProviders";

export function isLocalTranscribeProvider(provider: string): boolean {
  return provider === "local" || provider === "local_whisper";
}

export function isTranscribeEndpointReady(entry: TranscribeTierEntry, models: WhisperModel[], keys: ApiKeyConfig[]): boolean {
  if (!entry.model.trim() && !transcribeProviders[entry.provider]?.modelOptional) return false;
  if (isLocalTranscribeProvider(entry.provider)) return models.some(model => model.id === entry.model && model.downloaded);
  const key = keys.find(key => key.id === entry.apiKeyId);
  if (key?.capabilities && !supportsCapability(key, "stt")) return false;
  return (entry.provider === "custom" || entry.provider === "custom_whisper") ? Boolean(key?.apiUrl?.trim()) : Boolean(key?.apiKey?.trim());
}

export type TranscriptionSettings = Omit<TranscribeConfig, "provider" | "model" | "api_key" | "api_url" | "use_gpu">;

/** Resolve credentials and gate local-only features in one place. */
export function buildTranscriptionRequest(settings: TranscriptionSettings, entry: TranscribeTierEntry, keys: ApiKeyConfig[]): TranscribeConfig {
  const local = isLocalTranscribeProvider(entry.provider);
  const key = local ? undefined : keys.find(key => key.id === entry.apiKeyId);
  return {
    ...settings,
    provider: entry.provider,
    model: entry.model,
    api_key: key?.apiKey?.trim() || null,
    api_url: key?.apiUrl?.trim() || transcribeProviders[entry.provider]?.defaultUrl || null,
    quality: local && settings.quality,
    vad: local && settings.vad,
    use_gpu: local,
  };
}

export type TranscriptionOutcome =
  | { status: "success"; result: TranscribeStartResult }
  | { status: "cancelled" }
  | { status: "failed"; error: string };

export interface TranscriptionRunOptions {
  tiers: TranscribeTier[];
  models: WhisperModel[];
  keys: ApiKeyConfig[];
  settings: TranscriptionSettings;
  start: (config: TranscribeConfig) => Promise<TranscribeStartResult>;
  isCancelled: () => boolean;
  onTier?: (index: number, readyEntries: number) => void;
  onAttempt?: (entry: TranscribeTierEntry) => void;
  onFailure?: (entry: TranscribeTierEntry, error: string) => void;
}

/** Sequential failover. A cancelled run never starts another endpoint or publishes late results. */
export async function runTranscription(options: TranscriptionRunOptions): Promise<TranscriptionOutcome> {
  let lastError = "";
  for (const [index, tier] of options.tiers.entries()) {
    if (options.isCancelled()) return { status: "cancelled" };
    const ready = tier.entries.filter(entry => isTranscribeEndpointReady(entry, options.models, options.keys));
    if (!ready.length) continue;
    options.onTier?.(index, ready.length);
    for (const entry of ready) {
      if (options.isCancelled()) return { status: "cancelled" };
      options.onAttempt?.(entry);
      try {
        const result = await options.start(buildTranscriptionRequest(options.settings, entry, options.keys));
        if (options.isCancelled()) return { status: "cancelled" };
        if (!result.success) throw new Error(result.message);
        return { status: "success", result };
      } catch (error) {
        if (options.isCancelled()) return { status: "cancelled" };
        lastError = String(error);
        options.onFailure?.(entry, lastError);
      }
    }
  }
  return options.isCancelled() ? { status: "cancelled" } : { status: "failed", error: lastError };
}

/** Override local endpoints for this run while preserving cloud priority and saved tiers. */
export function withLocalModelOverride(tiers: TranscribeTier[], model: string | null): TranscribeTier[] {
  if (!model) return tiers;
  const mapped = tiers.map(tier => ({ ...tier, entries: tier.entries.map(entry =>
    isLocalTranscribeProvider(entry.provider) ? { ...entry, model } : entry
  ) }));
  if (mapped.some(tier => tier.entries.some(entry => isLocalTranscribeProvider(entry.provider)))) return mapped;
  return [{ id: "local-override", entries: [{ id: "local-override", provider: "local", model, apiKeyId: "" }] }, ...mapped];
}
