import { afterEach, expect, it, vi } from "vitest";
import { RequirementFeedback } from "./requirementFeedback.svelte";
afterEach(() => vi.useRealTimers());
it("new guidance replaces the old pulse and old timers cannot clear it", () => {
 vi.useFakeTimers(); const feedback = new RequirementFeedback<"files" | "models">();
 feedback.pulse(["files"]); vi.advanceTimersByTime(1000); feedback.pulse(["models"]);
 expect(feedback.classFor("files")).toBe(""); vi.advanceTimersByTime(800); expect(feedback.classFor("models")).toBe("requirement-pulse");
 vi.advanceTimersByTime(1000); expect(feedback.classFor("models")).toBe("");
 feedback.pulse(["files"]); feedback.clear(); expect(vi.getTimerCount()).toBe(0);
});
