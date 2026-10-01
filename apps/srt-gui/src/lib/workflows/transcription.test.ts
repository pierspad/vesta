import { expect, it, vi } from "vitest";
import { runTranscription, buildTranscriptionRequest, type TranscriptionRunOptions } from "./transcription";
import type { ApiKeyConfig } from "$lib/config/apiKeys";
const local = { id: "l", provider: "local", model: "base", apiKeyId: "" };
const cloud = { id: "c", provider: "groq", model: "whisper-large-v3", apiKeyId: "k" };
const success = { success: true, message: "ok" };
function options(): TranscriptionRunOptions {
 return { tiers: [{ id: "tier", entries: [local, cloud] }], models: [{ id: "base", downloaded: true, name: "base", size: "", speed: "" }], keys: [{ id: "k", apiKey: " token ", apiUrl: " https://example.com " }] as ApiKeyConfig[], settings: { input_path: "film.mkv", output_path: "film.en.srt", language: "en", translate_to_english: false, word_timestamps: true, max_segment_length: 30, quality: true, vad: true, vad_model_id: "silero", vad_custom_path: null }, start: vi.fn().mockResolvedValue(success), isCancelled: () => false };
}
it("keeps credentials and quality/VAD settings scoped to the appropriate provider", () => {
 const run = options();
 expect(buildTranscriptionRequest(run.settings, local, run.keys)).toMatchObject({ api_key: null, api_url: null, quality: true, vad: true, use_gpu: true });
 expect(buildTranscriptionRequest(run.settings, cloud, run.keys)).toMatchObject({ api_key: "token", api_url: "https://example.com", quality: false, vad: false, use_gpu: false });
});
it("skips unavailable local models and stops at the first usable endpoint", async () => {
 const run = options(); run.models = [];
 expect(await runTranscription(run)).toEqual({ status: "success", result: success });
 expect(run.start).toHaveBeenCalledExactlyOnceWith(expect.objectContaining({ provider: "groq" }));
});
it("falls back after rejected requests and unsuccessful responses", async () => {
 for (const first of [() => Promise.reject(new Error("offline")), () => Promise.resolve({ success: false, message: "bad audio" })]) {
 const run = options(); run.start = vi.fn().mockImplementationOnce(first).mockResolvedValue(success);
 expect((await runTranscription(run)).status).toBe("success"); expect(run.start).toHaveBeenCalledTimes(2);
 }
});
it("never retries or publishes a late result after cancellation", async () => {
 for (const reject of [false, true]) {
 let cancelled = false; const run = options(); run.isCancelled = () => cancelled;
 run.start = vi.fn(async () => { cancelled = true; if (reject) throw new Error("cancelled"); return success; });
 expect(await runTranscription(run)).toEqual({ status: "cancelled" }); expect(run.start).toHaveBeenCalledOnce();
 }
});
it("does not start cancelled work and reports exhausted failover", async () => {
 const run = options(); run.isCancelled = () => true;
 expect((await runTranscription(run)).status).toBe("cancelled"); expect(run.start).not.toHaveBeenCalled();
 run.isCancelled = () => false; run.start = vi.fn().mockRejectedValue(new Error("offline"));
 expect(await runTranscription(run)).toEqual({ status: "failed", error: "Error: offline" });
});
