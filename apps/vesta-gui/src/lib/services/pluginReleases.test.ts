import { expect, it, vi } from "vitest";
import type { InstalledPlugin } from "./pluginClient";
import { githubReleaseEndpoint, newerPluginRelease } from "./pluginReleases";
import { fetch } from "$lib/services/tauriHttp";

vi.mock("$lib/services/tauriHttp", () => ({ fetch: vi.fn() }));

const plugin = { sourceUrl: "https://github.com/example/movie-plugin.git", version: "1.9.0" } as InstalledPlugin;

it("checks repository installs and keeps pinned branches or local paths on their source", () => {
  expect(githubReleaseEndpoint(plugin)).toBe("https://api.github.com/repos/example/movie-plugin/releases/latest");
  expect(githubReleaseEndpoint({ ...plugin, sourceRef: "nightly" })).toBeUndefined();
  expect(githubReleaseEndpoint({ ...plugin, sourceRef: "v1.9.0" })).toBeDefined();
  expect(githubReleaseEndpoint({ ...plugin, sourceUrl: "/tmp/github.com/example/movie-plugin" })).toBeUndefined();
});

it("offers a newer stable release and ignores older releases, drafts and prereleases", async () => {
  const release = (tag_name: string, extra = {}) =>
    ({ ok: true, status: 200, json: async () => ({ tag_name, ...extra }) }) as Awaited<ReturnType<typeof fetch>>;
  vi.mocked(fetch)
    .mockResolvedValueOnce(release("v1.10.0"))
    .mockResolvedValueOnce(release("v1.8.0"))
    .mockResolvedValueOnce(release("v2.0.0", { draft: true }))
    .mockResolvedValueOnce(release("v2.0.0-beta.1"));
  await expect(newerPluginRelease(plugin)).resolves.toBe("v1.10.0");
  for (let i = 0; i < 3; i++) await expect(newerPluginRelease(plugin)).resolves.toBeUndefined();
});
