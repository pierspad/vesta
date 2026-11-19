import { isBuiltinEndpointId, supportsCapability, type ApiCapability, type ApiKeyConfig } from "$lib/config/apiKeys";
import { providers } from "$lib/config/llmProviders";

export type EndpointUsageChoice = "llm" | "stt" | "itt" | "both" | "llm_itt" | "all";

export function capabilitiesToUsageChoice(key: ApiKeyConfig): EndpointUsageChoice {
  const hasLlm = supportsCapability(key, "llm");
  const hasStt = supportsCapability(key, "stt");
  const hasItt = supportsCapability(key, "itt");
  if (hasLlm && hasStt && hasItt) return "all";
  if (hasLlm && hasStt) return "both";
  if (hasLlm && hasItt) return "llm_itt";
  if (hasStt) return "stt";
  if (hasItt) return "itt";
  return "llm";
}

export function usageChoiceToCapabilities(choice: EndpointUsageChoice): ApiCapability[] {
  if (choice === "all") return ["llm", "stt", "itt"];
  if (choice === "both") return ["llm", "stt"];
  if (choice === "llm_itt") return ["llm", "itt"];
  return [choice];
}

/** Provider catalogs + doc/signup links for the add/edit API key modal.
 * Static data, only ever consumed by the api-key-editing feature (the
 * modal component and this store's own openAdd), so they live here rather
 * than in a shared providers module. */
export const llmProviderIds = ["local", "google", "groq", "openai", "anthropic", "openrouter", "mistral", "nvidia", "custom"];
export const whisperProviderIds = ["groq", "openai", "deepgram", "assemblyai", "custom"];

export const llmApiKeyUrls: Record<string, string> = {
  google: "https://aistudio.google.com/apikey",
  groq: "https://console.groq.com/keys",
  openai: "https://platform.openai.com/api-keys",
  anthropic: "https://console.anthropic.com/settings/keys",
  openrouter: "https://openrouter.ai/keys",
  mistral: "https://console.mistral.ai/api-keys",
  nvidia: "https://build.nvidia.com",
};

export const llmDocUrls: Record<string, string> = {
  google: "https://ai.google.dev/gemini-api/docs",
  groq: "https://console.groq.com/docs",
  openai: "https://developers.openai.com/api/docs",
  anthropic: "https://docs.anthropic.com",
  openrouter: "https://openrouter.ai/docs",
  mistral: "https://docs.mistral.ai",
  nvidia: "https://docs.api.nvidia.com/nim/reference/llm-apis",
};

export const whisperApiKeyUrls: Record<string, string> = {
  groq: "https://console.groq.com/keys",
  openai: "https://platform.openai.com/api-keys",
  deepgram: "https://console.deepgram.com",
  assemblyai: "https://www.assemblyai.com/app/api-keys",
};

export const whisperDocUrls: Record<string, string> = {
  groq: "https://console.groq.com/docs/speech-to-text",
  openai: "https://developers.openai.com/api/docs/guides/speech-to-text",
  deepgram: "https://developers.deepgram.com/reference/speech-to-text/listen-pre-recorded",
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
  modalContext = $state<"llm" | "whisper" | "itt">("llm");
  newKeyType = $state<ApiKeyConfig["apiType"]>("google");
  newKeyName = $state("");
  newKeyValue = $state("");
  newKeyUrl = $state("");
  newKeyUsage = $state<EndpointUsageChoice>("llm");
  showNewKeyPassword = $state(false);

  openAdd(context: "llm" | "whisper" | "itt", providerId?: string) {
    this.editKeyId = null;
    this.modalContext = context;
    this.newKeyUsage = context === "whisper" ? "stt" : context === "itt" ? "itt" : "llm";
    const allowedIds = context === "whisper" ? whisperProviderIds : llmProviderIds;
    const defaultProviderId = context === "whisper" ? "groq" : context === "itt" ? "custom" : "google";

    const normalizedProviderId =
      providerId && allowedIds.includes(providerId) ? providerId : defaultProviderId;

    if (normalizedProviderId) {
      this.newKeyType = normalizedProviderId as ApiKeyConfig["apiType"];
      this.newKeyName = normalizedProviderId === "openai" ? "OpenAI" : providers[normalizedProviderId]?.name || "";
    }
    this.newKeyValue = "";
    this.newKeyUrl = this.newKeyType === "local" ? providers.local.defaultApiUrl || "" : "";
    if (this.newKeyType === "local") this.newKeyUsage = context === "itt" ? "itt" : "llm";
    if (this.newKeyType === "custom") this.newKeyName = "";
    this.showAddKey = true;
  }

  openEdit(key: ApiKeyConfig, context: "llm" | "whisper" | "itt") {
    this.editKeyId = key.id;
    this.modalContext = context;
    this.newKeyType = key.apiType === "custom_whisper" ? "custom" : key.apiType;
    this.newKeyUsage = capabilitiesToUsageChoice(key);
    this.newKeyName = key.name;
    this.newKeyValue = key.apiKey;
    this.newKeyUrl = key.apiUrl || "";
    this.showNewKeyPassword = false;
    this.showAddKey = true;
  }

  selectProvider(pid: string) {
    if (this.editKeyId) return;
    if (isBuiltinEndpointId(this.editKeyId) && ((this.editKeyId === "local" && pid !== "local") || (this.editKeyId !== "local" && pid !== "custom"))) return;
    const isCustom = pid === "custom";
    const prov = providers[pid];
    this.newKeyType = pid as ApiKeyConfig["apiType"];
    this.newKeyName = isCustom ? "" : pid === "openai" ? "OpenAI" : prov?.name || pid;
    this.newKeyUrl = isCustom ? "" : prov?.defaultApiUrl || "";
    this.newKeyValue = "";
    if (pid === "local") this.newKeyUsage = this.modalContext === "itt" ? "itt" : "llm";
    else if (isCustom) this.newKeyUsage = this.modalContext === "whisper" ? "stt" : this.modalContext === "itt" ? "itt" : "llm";
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
