import { expect, it } from "vitest";
import { apiKeyDefaultGroup, normalizeApiKeyDefaults, loadAndValidateApiKeys, supportsCapability, type ApiKeyConfig } from "./apiKeys";
import * as config from "$lib/config/vestaConfig";
const key=(id:string,isDefault?:boolean):ApiKeyConfig=>({id,name:id,apiType:"openai",apiKey:"example",isDefault});
it("selects the sole configuration and preserves a chosen default after adding another",()=>{
 expect(normalizeApiKeyDefaults([key("first")])[0].isDefault).toBe(true);
 expect(normalizeApiKeyDefaults([key("first",true),key("second")]).map(k=>k.isDefault)).toEqual([true,false]);
 expect(normalizeApiKeyDefaults([key("first"),key("second",true)]).map(k=>k.isDefault)).toEqual([false,true]);
});
it("repairs missing and duplicate legacy defaults without mutating stored objects",()=>{
 const keys=[key("first",true),key("second",true)];
 expect(normalizeApiKeyDefaults(keys).map(k=>k.isDefault)).toEqual([true,false]);
 expect(keys[1].isDefault).toBe(true);
 expect(normalizeApiKeyDefaults([key("first"),key("second")]).map(k=>k.isDefault)).toEqual([true,false]);
 expect(normalizeApiKeyDefaults([key("second")])[0].isDefault).toBe(true);
});
it("keeps custom speech and text configurations in separate default groups",()=>{
 const speech:ApiKeyConfig={...key("speech"),apiType:"custom",capabilities:["stt"]};
 const text:ApiKeyConfig={...key("text"),apiType:"custom",capabilities:["llm"]};
 expect(apiKeyDefaultGroup(speech)).not.toBe(apiKeyDefaultGroup(text));
 expect(normalizeApiKeyDefaults([speech,text]).every(k=>k.isDefault)).toBe(true);
});
it("repairs defaults when loading existing settings and builtin endpoints",()=>{
 config.setItem("srt-tools-api-keys",JSON.stringify([key("saved")]));
 const loaded=loadAndValidateApiKeys();
 expect(loaded.find(k=>k.id==="saved")?.isDefault).toBe(true);
 expect(loaded.find(k=>k.id==="local")?.isDefault).toBe(true);
 config.removeItem("srt-tools-api-keys");
});
 it("preserves anthropic keys on load and supports only llm capability",()=>{
 const anthropicKey:ApiKeyConfig={id:"ant-1",name:"Anthropic",apiType:"anthropic",apiKey:"sk-ant-test"};
 config.setItem("srt-tools-api-keys",JSON.stringify([anthropicKey]));
 const loaded=loadAndValidateApiKeys();
 const found=loaded.find(k=>k.id==="ant-1");
 expect(found?.apiType).toBe("anthropic");
 expect(found?.isDefault).toBe(true);
 expect(supportsCapability(anthropicKey,"llm")).toBe(true);
 expect(supportsCapability(anthropicKey,"stt")).toBe(false);
 config.removeItem("srt-tools-api-keys");
});
it("discards retired github keys on load while preserving valid keys and builtin endpoints",()=>{
 const validKey:ApiKeyConfig={id:"or-1",name:"OpenRouter",apiType:"openrouter",apiKey:"sk-or-test"};
 const retiredKey={id:"gh-1",name:"GitHub Models",apiType:"github",apiKey:"github_pat_test"};
 config.setItem("srt-tools-api-keys",JSON.stringify([retiredKey,validKey]));
 const loaded=loadAndValidateApiKeys();
 expect(loaded.find(k=>k.id==="gh-1")).toBeUndefined();
 expect(loaded.find(k=>k.id==="or-1")?.apiType).toBe("openrouter");
 expect(loaded.find(k=>k.id==="local")?.apiType).toBe("local");
 expect(loaded.find(k=>k.id==="local_stt_server")?.apiType).toBe("custom");
 config.removeItem("srt-tools-api-keys");
});
