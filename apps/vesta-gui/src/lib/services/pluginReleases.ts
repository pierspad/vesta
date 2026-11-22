import type { InstalledPlugin } from "$lib/services/pluginClient";
import { fetch } from "$lib/services/tauriHttp";
import { compareVersions } from "$lib/utils/updateVersion";

const stableTag = /^v\d+\.\d+\.\d+$/;

export function githubReleaseEndpoint(plugin: InstalledPlugin): string | undefined {
  if (plugin.sourceRef && plugin.sourceRef !== "HEAD" && !stableTag.test(plugin.sourceRef)) return;
  const repo = plugin.sourceUrl.trim().match(/^(?:https?:\/\/|git@)?github\.com[/:]([\w.-]+)\/([\w.-]+?)\/?$/i);
  if (!repo) return;
  const name = repo[2].replace(/\.git$/i, "");
  if (!name || [repo[1], name].some(segment => segment === "." || segment === "..")) return;
  return `https://api.github.com/repos/${repo[1]}/${name}/releases/latest`;
}

export async function newerPluginRelease(plugin: InstalledPlugin): Promise<string | undefined> {
  const url = githubReleaseEndpoint(plugin);
  if (!url) return;
  const response = await fetch(url, { timeoutMs: 8000 });
  if (response.status === 404) return;
  if (!response.ok) throw new Error(`GitHub HTTP ${response.status}`);
  const release = await response.json() as { tag_name?: string; draft?: boolean; prerelease?: boolean };
  const tag = release?.tag_name;
  if (release?.draft || release?.prerelease || !tag || !stableTag.test(tag)) return;
  if ((compareVersions(tag, plugin.version) ?? 0) > 0) return tag;
}
