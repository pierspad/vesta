import { expect, it } from "vitest";
import { anchorOffsetSpread, initialSyncCheckpoints } from "./syncAssessment";
it("reports agreement rather than inventing match confidence from anchor density", () => {
 expect(anchorOffsetSpread(Array.from({ length: 4 }, () => ({ offset_ms: 0 })))).toBe(0);
 expect(anchorOffsetSpread([{ offset_ms: -200 }, { offset_ms: 300 }])).toBe(500);
 expect(anchorOffsetSpread([])).toBeNull();
 expect(anchorOffsetSpread([{ offset_ms: 0 }])).toBeNull();
});
it("bounds checkpoints for short files and includes the end of long files", () => {
 expect(initialSyncCheckpoints(0)).toEqual([]);
 expect(initialSyncCheckpoints(1)).toEqual([1]);
 expect(initialSyncCheckpoints(2)).toEqual([1, 2]);
 expect(initialSyncCheckpoints(1755)).toEqual([1, 585, 1170, 1755]);
});
