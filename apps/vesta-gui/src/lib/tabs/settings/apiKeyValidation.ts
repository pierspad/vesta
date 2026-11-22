import { providers } from "$lib/config/llmProviders";
import { hasProviderNameConflict, type ApiCapability, type ApiKeyConfig } from "$lib/config/apiKeys";
import { usageChoiceToCapabilities, type EndpointUsageChoice } from "$lib/stores/apiKeyEditorStore.svelte";

export interface ApiKeyDraftInput {
  editKeyId: string | null;
  newKeyType: ApiKeyConfig["apiType"];
  newKeyName: string;
  newKeyValue: string;
  newKeyUrl: string;
  newKeyUsage: EndpointUsageChoice;
}

export type ApiKeyValidationResult =
  | { ok: true; nextKeys: ApiKeyConfig[]; resolvedUrl?: string; isEdit: boolean }
  | { ok: false; errorKey: string };

export function validateAndApplyApiKey(
  existingKeys: ApiKeyConfig[],
  draft: ApiKeyDraftInput,
  generateId: () => string,
): ApiKeyValidationResult {
  const { editKeyId, newKeyType, newKeyName, newKeyValue, newKeyUrl, newKeyUsage } = draft;
  const capabilities: ApiCapability[] | undefined =
    newKeyType === "custom" || newKeyType === "local"
      ? usageChoiceToCapabilities(newKeyUsage)
      : undefined;

  if (!newKeyName.trim()) return { ok: false, errorKey: "settings.errorNameRequired" };
  if ((newKeyType === "local" || newKeyType === "custom") && !newKeyUrl.trim()) {
    return { ok: false, errorKey: "settings.errorEndpointRequired" };
  }
  if (newKeyType !== "local" && newKeyType !== "custom" && !newKeyValue.trim()) {
    return { ok: false, errorKey: "settings.errorKeyRequired" };
  }
  if (newKeyType === "google" && !newKeyValue.trim().startsWith("AIza")) {
    return { ok: false, errorKey: "settings.errorInvalidGoogleKey" };
  }
  if (newKeyType === "groq" && !newKeyValue.trim().startsWith("gsk_")) {
    return { ok: false, errorKey: "settings.errorInvalidGroqKey" };
  }
  if (hasProviderNameConflict(existingKeys, newKeyName, editKeyId)) {
    return { ok: false, errorKey: "settings.errorDuplicateCustomName" };
  }
  if (newKeyType === "custom" || newKeyType === "local") {
    try {
      const endpoint = new URL(newKeyUrl.trim());
      if (
        !["http:", "https:"].includes(endpoint.protocol) ||
        endpoint.username ||
        endpoint.password ||
        endpoint.search ||
        endpoint.hash
      ) {
        return { ok: false, errorKey: "settings.errorEndpointInvalid" };
      }
    } catch {
      return { ok: false, errorKey: "settings.errorEndpointInvalid" };
    }
  }

  let resolvedUrl = newKeyUrl.trim() || undefined;
  if (newKeyType !== "custom" && newKeyType !== "local") {
    resolvedUrl = providers[newKeyType]?.defaultApiUrl || resolvedUrl;
  }

  if (editKeyId) {
    const nextKeys = existingKeys.map((k) =>
      k.id === editKeyId
        ? {
            ...k,
            name: newKeyName.trim(),
            apiType: newKeyType,
            apiKey: newKeyValue.trim(),
            apiUrl: resolvedUrl,
            capabilities,
          }
        : k,
    );
    return { ok: true, nextKeys, resolvedUrl, isEdit: true };
  }

  const newKey: ApiKeyConfig = {
    id: generateId(),
    name: newKeyName.trim(),
    apiType: newKeyType,
    apiKey: newKeyValue.trim(),
    apiUrl: resolvedUrl,
    capabilities,
    isDefault: existingKeys.filter((k) => k.apiType === newKeyType).length === 0,
  };
  return { ok: true, nextKeys: [...existingKeys, newKey], resolvedUrl, isEdit: false };
}
