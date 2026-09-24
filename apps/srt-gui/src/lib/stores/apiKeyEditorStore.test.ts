import { describe, it, expect, beforeEach } from "vitest";
import {
  apiKeyEditorStore,
  llmProviderIds,
  whisperProviderIds,
  llmApiKeyUrls,
  whisperApiKeyUrls,
} from "./apiKeyEditorStore.svelte";

describe("apiKeyEditorStore", () => {
  beforeEach(() => {
    apiKeyEditorStore.reset();
  });

  describe("catalogs & links", () => {
    it("defines valid catalogs for LLM and Whisper providers", () => {
      expect(llmProviderIds).toContain("google");
      expect(llmProviderIds).toContain("openai");
      expect(llmProviderIds).toContain("groq");
      expect(llmProviderIds).not.toContain("github");

      expect(whisperProviderIds).toContain("groq");
      expect(whisperProviderIds).toContain("openai");
      expect(whisperProviderIds).toContain("deepgram");
      expect(whisperProviderIds).toContain("assemblyai");
    });

    it("has valid HTTPS URL endpoints for all listed providers", () => {
      for (const [provider, url] of Object.entries(llmApiKeyUrls)) {
        expect(url.startsWith("https://")).toBe(true);
        expect(provider.length).toBeGreaterThan(0);
      }
      for (const [provider, url] of Object.entries(whisperApiKeyUrls)) {
        expect(url.startsWith("https://")).toBe(true);
        expect(provider.length).toBeGreaterThan(0);
      }
    });
  });

  describe("modal state operations", () => {
    it("opens add modal for LLM with default google provider", () => {
      apiKeyEditorStore.openAdd("llm");
      expect(apiKeyEditorStore.showAddKey).toBe(true);
      expect(apiKeyEditorStore.modalContext).toBe("llm");
      expect(apiKeyEditorStore.newKeyType).toBe("google");
      expect(apiKeyEditorStore.editKeyId).toBeNull();
    });

    it("opens add modal for Whisper with default groq provider", () => {
      apiKeyEditorStore.openAdd("whisper");
      expect(apiKeyEditorStore.showAddKey).toBe(true);
      expect(apiKeyEditorStore.modalContext).toBe("whisper");
      expect(apiKeyEditorStore.newKeyType).toBe("groq");
    });

    it("opens edit modal with existing API key data", () => {
      apiKeyEditorStore.openEdit(
        {
          id: "key-123",
          apiType: "openai",
          name: "My OpenAI Key",
          apiKey: "sk-proj-1234567890",
          apiUrl: "https://api.openai.com/v1",
        },
        "llm",
      );

      expect(apiKeyEditorStore.showAddKey).toBe(true);
      expect(apiKeyEditorStore.editKeyId).toBe("key-123");
      expect(apiKeyEditorStore.newKeyType).toBe("openai");
      expect(apiKeyEditorStore.newKeyName).toBe("My OpenAI Key");
      expect(apiKeyEditorStore.newKeyValue).toBe("sk-proj-1234567890");
      expect(apiKeyEditorStore.newKeyUrl).toBe("https://api.openai.com/v1");
    });

    it("switches provider dynamically", () => {
      apiKeyEditorStore.openAdd("llm");
      apiKeyEditorStore.selectProvider("mistral");
      expect(apiKeyEditorStore.newKeyType).toBe("mistral");

      apiKeyEditorStore.selectProvider("custom");
      expect(apiKeyEditorStore.newKeyType).toBe("custom");
      expect(apiKeyEditorStore.newKeyName).toBe("");
      expect(apiKeyEditorStore.newKeyUrl).toBe("");
    });

    it("resets state on reset() and close()", () => {
      apiKeyEditorStore.openAdd("llm");
      apiKeyEditorStore.newKeyValue = "test-key";
      apiKeyEditorStore.close();
      expect(apiKeyEditorStore.showAddKey).toBe(false);

      apiKeyEditorStore.reset();
      expect(apiKeyEditorStore.newKeyValue).toBe("");
      expect(apiKeyEditorStore.newKeyName).toBe("");
      expect(apiKeyEditorStore.editKeyId).toBeNull();
    });
  });
});
