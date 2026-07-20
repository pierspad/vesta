import type { ApiKeyConfig } from "$lib/config/apiKeys";
import { providers } from "$lib/config/llmProviders";

/** Provider catalogs + doc/signup links for the add/edit API key modal.
 * Static data, only ever consumed by the api-key-editing feature (the
 * modal component and this store's own openAdd), so they live here rather
 * than in a shared providers module. */
export const llmProviderIds = ["google", "groq", "openai", "openrouter", "mistral", "github", "nvidia", "custom"];
export const whisperProviderIds = ["groq", "openai", "deepgram", "assemblyai", "custom"];

export const llmApiKeyUrls: Record<string, string> = {
  google: "https://aistudio.google.com/apikey",
  groq: "https://console.groq.com/keys",
  openai: "https://platform.openai.com/api-keys",
  openrouter: "https://openrouter.ai/keys",
  mistral: "https://console.mistral.ai/api-keys",
  github: "https://github.com/settings/personal-access-tokens",
  nvidia: "https://build.nvidia.com",
};

export const llmDocUrls: Record<string, string> = {
  google: "https://ai.google.dev/gemini-api/docs",
  groq: "https://console.groq.com/docs",
  openai: "https://platform.openai.com/docs",
  openrouter: "https://openrouter.ai/docs",
  mistral: "https://docs.mistral.ai",
  github: "https://docs.github.com",
  nvidia: "https://docs.nvidia.com",
};

export const whisperApiKeyUrls: Record<string, string> = {
  groq: "https://console.groq.com/keys",
  openai: "https://platform.openai.com/api-keys",
  deepgram: "https://console.deepgram.com",
  assemblyai: "https://www.assemblyai.com/app/api-keys",
};

export const whisperDocUrls: Record<string, string> = {
  groq: "https://console.groq.com/docs/speech-to-text",
  openai: "https://platform.openai.com/guides/speech-to-text",
  deepgram: "https://developers.deepgram.com/docs/deepgram-whisper-cloud",
  assemblyai: "https://www.assemblyai.com/docs",
};

/** Transient state for the add/edit API key modal (SettingsTab.svelte's
 * "llm"/"whisper" sections both trigger it, so ownership can't live inside
 * either section's own component — same rationale as previewStore /
 * episodeMediaEditorStore in the FlashcardsTab.svelte refactor). Actually
 * persisting a key (mutating the `apiKeys` array + localStorage) stays in
 * SettingsTab.svelte, which is the sole owner of that domain data. */
class ApiKeyEditorStore {
  showAddKey = $state(false);
  editKeyId = $state<string | null>(null);
  modalContext = $state<"llm" | "whisper">("llm");
  newKeyType = $state<ApiKeyConfig["apiType"]>("google");
  newKeyName = $state("");
  newKeyValue = $state("");
  newKeyUrl = $state("");
  showNewKeyPassword = $state(false);

  openAdd(context: "llm" | "whisper", providerId?: string) {
    this.editKeyId = null;
    this.modalContext = context;
    const allowedIds = context === "whisper" ? whisperProviderIds : llmProviderIds;
    const defaultProviderId = context === "whisper" ? "groq" : "google";

    const normalizedProviderId =
      providerId && allowedIds.includes(providerId) ? providerId : defaultProviderId;

    if (normalizedProviderId) {
      this.newKeyType = normalizedProviderId as ApiKeyConfig["apiType"];
      this.newKeyName =
        normalizedProviderId === "openai" ? "Open AI" : providers[normalizedProviderId]?.name || "";
    }
    this.newKeyValue = "";
    this.newKeyUrl = this.newKeyType === "local" ? providers.local.defaultApiUrl || "" : "";
    if (this.newKeyType === "custom") this.newKeyName = "";
    this.showAddKey = true;
  }

  openEdit(key: ApiKeyConfig, context: "llm" | "whisper") {
    this.editKeyId = key.id;
    this.modalContext = context;
    this.newKeyType = key.apiType;
    this.newKeyName = key.name;
    this.newKeyValue = key.apiKey;
    this.newKeyUrl = key.apiUrl || "";
    this.showNewKeyPassword = false;
    this.showAddKey = true;
  }

  selectProvider(pid: string) {
    const isCustom = pid === "custom";
    const prov = providers[pid];
    this.newKeyType = pid as ApiKeyConfig["apiType"];
    this.newKeyName = isCustom ? "" : pid === "openai" ? "Open AI" : prov?.name || pid;
    this.newKeyUrl = isCustom ? "" : prov?.defaultApiUrl || "";
    this.newKeyValue = "";
  }

  close() {
    this.showAddKey = false;
  }

  /** Called by SettingsTab.svelte after a successful save. */
  reset() {
    this.newKeyName = "";
    this.newKeyValue = "";
    this.newKeyUrl = "";
    this.editKeyId = null;
    this.showAddKey = false;
  }
}

export const apiKeyEditorStore = new ApiKeyEditorStore();
