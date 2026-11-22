// @vitest-environment happy-dom
import { afterEach, expect, it, vi } from "vitest";
import { mount, tick, unmount } from "svelte";
import MatchDiagnosticsModal from "./MatchDiagnosticsModal.svelte";
import { DEFAULT_SMART_MATCHING_RULES as rules } from "$lib/stores/smartMatchingStore.svelte";
import { mergeSeriesDroppedFiles } from "$lib/utils/seriesFileMatching";
const mounted: ReturnType<typeof mount>[] = [];
afterEach(async () => {
  for (const component of mounted.splice(0)) await unmount(component);
  document.body.replaceChildren();
  vi.restoreAllMocks();
});
async function render(subtitle: string, media: string) {
  vi.spyOn(HTMLDialogElement.prototype, "showModal").mockImplementation(() => {});
  const episode = mergeSeriesDroppedFiles([], [subtitle], [media], rules, "en", "it")[0];
  const target = document.createElement("div");
  document.body.append(target);
  mounted.push(mount(MatchDiagnosticsModal, { target, props: {
    episode, focusedPath: subtitle, onclose: vi.fn(), onrules: vi.fn(),
  } }));
  await tick();
  return { target, episode };
}
it("explains the actual filename match without claiming that an episode regex fired", async () => {
  const { target } = await render("/detour_clip_90s_it.srt", "/detour_clip_90s.mp4");
  expect(target.textContent).toContain("Matched by filename");
  expect(target.textContent).toContain("No episode regex was used");
  expect(target.textContent).not.toContain("Applied rule");
  expect(target.querySelector("h2")?.textContent).toContain("Match details");
  expect(target.querySelector('[data-focused="true"]')?.textContent).toContain("detour_clip_90s_it.srt");
});
it("shows the exact applied regex and highlights its capture in an episode match", async () => {
  const { target, episode } = await render("/Show.S01E02.en.srt", "/Show_1x02.mp4");
  expect(episode.matching?.method).toBe("episode");
  expect(target.textContent).toContain("Matched by episode");
  expect(target.textContent).toContain("Applied rule #1");
  expect(target.textContent).toContain(rules.episodeRegexes[0]);
  expect(target.querySelector("mark")?.textContent).toBe("S01E02");
});

it("does not attribute a manual replacement to an old automatic match", async () => {
  vi.spyOn(HTMLDialogElement.prototype, "showModal").mockImplementation(() => {});
  const episode = mergeSeriesDroppedFiles([], ["/detour_clip_90s_it.srt"], ["/detour_clip_90s.mp4"], rules, "en", "it")[0];
  const target = document.createElement("div");
  document.body.append(target);
  mounted.push(mount(MatchDiagnosticsModal, { target, props: {
    episode: { ...episode, nativeSubsPath: "/manual-reference.srt" }, onclose: vi.fn(), onrules: vi.fn(),
  } }));
  await tick();
  expect(target.textContent).not.toContain("Matched by filename");
  expect(target.textContent).toContain("No matching history");
});
