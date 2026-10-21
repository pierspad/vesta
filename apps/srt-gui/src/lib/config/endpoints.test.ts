import { beforeEach, expect, it, vi } from "vitest";
const { storage, fetchModels } = vi.hoisted(() => ({ storage: new Map<string, string>(), fetchModels: vi.fn() }));
vi.mock("$lib/config/vestaConfig", () => ({getItem: (key: string) => storage.get(key) ?? null, setItem: (key: string, value: string) => storage.set(key,value)}));
vi.mock("$lib/services/modelDiscovery", () => ({fetchModelsFromEndpoint: fetchModels}));
import { loadAndValidateApiKeys, supportsCapability, hasProviderNameConflict, type ApiKeyConfig } from "./apiKeys";
import { buildTiersPayload, checkTiersAvailability } from "./llmTiers";
import { isTranscribeEndpointReady } from "$lib/workflows/transcription";
beforeEach(() => {storage.clear(); fetchModels.mockReset();});
it("provides editable local endpoints and preserves saved URLs without duplicating IDs", () => {
 expect(loadAndValidateApiKeys().map(key => key.id)).toEqual(["local", "local_stt_server"]);
 storage.set("srt-tools-api-keys", JSON.stringify([{id:"local",name:"LM Studio",apiType:"local",apiKey:"",apiUrl:"http://localhost:1234/v1"}]));
 const keys = loadAndValidateApiKeys();
 expect(keys.filter(key => key.id === "local")).toHaveLength(1);
 expect(keys.find(key => key.id === "local")?.apiUrl).toBe("http://localhost:1234/v1");
 const payload = buildTiersPayload([{id:"t",entries:[{id:"e",provider:"local",apiKeyId:"",model:"actual-model"}]}], keys);
 expect(payload?.[0][0].api_url).toBe("http://localhost:1234/v1");
});
it("shares supported credentials while keeping custom capabilities explicit", () => {
 const key: ApiKeyConfig = {id:"key",name:"Key",apiType:"openai",apiKey:"test"};
 expect(supportsCapability(key,"llm")).toBe(true);expect(supportsCapability(key,"stt")).toBe(true);
 const custom = {...key,apiType:"custom" as const,capabilities:["stt" as const]};
 expect(supportsCapability(custom,"llm")).toBe(false);expect(supportsCapability(custom,"stt")).toBe(true);
});
it("allows custom local speech and LLM endpoints without fabricated API keys", async () => {
 const keys: ApiKeyConfig[] = [{id:"server",name:"Server",apiType:"custom",apiKey:"",apiUrl:"http://localhost:8000/v1",capabilities:["llm","stt"]}];
 expect(isTranscribeEndpointReady({id:"e",provider:"custom_whisper",apiKeyId:"server",model:"speech"},[],keys)).toBe(true);
 expect(isTranscribeEndpointReady({id:"e",provider:"custom_whisper",apiKeyId:"server",model:""},[],keys)).toBe(false);
 fetchModels.mockResolvedValue([{id:"model",name:"Model"}]);
 expect((await checkTiersAvailability([{id:"t",entries:[{id:"e",provider:"custom",apiKeyId:"server",model:"model"}]}],keys)).available).toBe(true);
});

it("permits a manual model when a reachable endpoint does not implement /models", async () => {
 const keys: ApiKeyConfig[] = [{id:"server",name:"Server",apiType:"custom",apiKey:"",apiUrl:"http://localhost:8000/v1"}];
 fetchModels.mockRejectedValue(Object.assign(new Error("HTTP 404"),{status:404}));
 expect((await checkTiersAvailability([{id:"t",entries:[{id:"e",provider:"custom",apiKeyId:"server",model:"manual-model"}]}],keys)).available).toBe(true);
});

it("rejects stale tiers after a custom endpoint capability is changed", async () => {
 const key: ApiKeyConfig = {id:"server",name:"Speech only",apiType:"custom",apiKey:"",apiUrl:"http://localhost:8000/v1",capabilities:["stt"]};
 const tiers = [{id:"t",entries:[{id:"e",provider:"custom" as const,apiKeyId:"server",model:"model"}]}];
 expect(buildTiersPayload(tiers,[key])).toBeNull();
 expect((await checkTiersAvailability(tiers,[key])).available).toBe(false);
 expect(isTranscribeEndpointReady(tiers[0].entries[0],[],[{...key,capabilities:["llm"]}])).toBe(false);
});

it("requires unique provider names across types on creation and rename", () => {
 const keys: ApiKeyConfig[] = [{id:"a",name:"My Provider",apiType:"openai",apiKey:"test"},{id:"b",name:"Other",apiType:"custom",apiKey:""}];
 expect(hasProviderNameConflict(keys,"  MY   provider ")).toBe(true);
 expect(hasProviderNameConflict(keys,"Ｍｙ Provider")).toBe(true);
 expect(hasProviderNameConflict(keys,"my provider","a")).toBe(false);
 expect(hasProviderNameConflict(keys,"my provider","b")).toBe(true);
 expect(hasProviderNameConflict(loadAndValidateApiKeys(),"Local LLM")).toBe(true);
});
