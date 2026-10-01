import { expect, it } from "vitest";
import { generateTranscriptionOutputPath, rewriteTranscriptionOutputLanguage, formatSubtitleTime } from "./transcriptionPaths";
import { formatElapsedTime } from "./elapsedTime";
it("preserves Unix and Windows directories while rewriting language suffixes", () => {
 expect(rewriteTranscriptionOutputLanguage("/films/movie.en.srt", "it")).toBe("/films/movie.it.srt");
 expect(rewriteTranscriptionOutputLanguage("C:\\films.en\\movie.en.srt", "ja")).toBe("C:\\films.en\\movie.ja.srt");
 expect(rewriteTranscriptionOutputLanguage("movie.srt", "ko")).toBe("movie.ko.srt");
 expect(rewriteTranscriptionOutputLanguage("movie.mp4", "it")).toBe("movie.mp4");
 expect(generateTranscriptionOutputPath("C:\\films.mkv\\movie.mp4", "auto")).toBe("C:\\films.mkv\\movie.auto.srt");
});
it("formats subtitle timestamps and elapsed run durations", () => {
 expect(formatSubtitleTime(3723456)).toBe("1:02:03.456"); expect(formatSubtitleTime(12345)).toBe("0:12.345");
 expect(formatElapsedTime(3723456)).toBe("01:02:03"); expect(formatElapsedTime(-1)).toBe("00:00:00");
});
