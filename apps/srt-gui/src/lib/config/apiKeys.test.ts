import { expect, it } from "vitest";
import { apiKeyDefaultGroup, normalizeApiKeyDefaults, loadAndValidateApiKeys, type ApiKeyConfig } from "./apiKeys";
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
