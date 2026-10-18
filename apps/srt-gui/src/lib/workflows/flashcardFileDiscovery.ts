import { areMatchingCompanions } from "$lib/utils/seriesFileMatching";
import type { SmartMatchingRules } from "$lib/stores/smartMatchingStore.svelte";

export interface FileDiscoveryOptions {
  enabled: boolean;
  rules?: SmartMatchingRules;
  targetLanguage: string;
  nativeLanguage: string;
  invoke: <T>(command: string, args?: Record<string, unknown>) => Promise<T>;
}

/** Suggestions are best-effort, bounded and returned in input order. */
async function discover<T, R>(items: T[], visit: (item: T) => Promise<R>): Promise<R[]> {
  const results: R[] = new Array(items.length);
  let next = 0;
  await Promise.all(Array.from({ length: Math.min(4, items.length) }, async () => {
    while (next < items.length) {
      const index = next++;
      results[index] = await visit(items[index]);
    }
  }));
  return results;
}

export async function expandFlashcardFiles(subtitleFiles: string[], mediaFiles: string[], options: FileDiscoveryOptions): Promise<{ subtitleFiles: string[]; mediaFiles: string[] }> {
  if (!options.enabled || (!subtitleFiles.length && !mediaFiles.length)) return { subtitleFiles, mediaFiles };
  const accepts = (a: string, b: string) => !options.rules || areMatchingCompanions(a, b, options.rules);
  const subtitles = new Set(subtitleFiles);
  const media = new Set(mediaFiles);
  const suggestions = await discover([...media], async path => {
    try {
      return await options.invoke<{ target: string | null; native: string | null }>("sync_suggest_subtitles_for_media", {
        mediaPath: path, defaultTargetLang: options.targetLanguage || null, defaultNativeLang: options.nativeLanguage || null,
      });
    } catch { return null; }
  });
  const mediaInputs = [...media];
  for (const [index, suggestion] of suggestions.entries()) {
    if (suggestion?.target && accepts(mediaInputs[index], suggestion.target)) subtitles.add(suggestion.target);
    if (suggestion?.native && accepts(mediaInputs[index], suggestion.native)) subtitles.add(suggestion.native);
  }
  const companions = await discover([...subtitles], async path => {
    const [subtitle, mediaPath] = await Promise.all([
      options.invoke<string | null>("sync_suggest_companion_subtitle_for_srt", { srtPath: path }).catch(() => null),
      options.invoke<string | null>("sync_suggest_media_for_srt", { srtPath: path }).catch(() => null),
    ]);
    return { subtitle, mediaPath };
  });
  const subtitleInputs = [...subtitles];
  for (const [index, companion] of companions.entries()) {
    if (companion.subtitle && accepts(subtitleInputs[index], companion.subtitle)) subtitles.add(companion.subtitle);
    if (companion.mediaPath && accepts(subtitleInputs[index], companion.mediaPath)) media.add(companion.mediaPath);
  }
  return { subtitleFiles: [...subtitles], mediaFiles: [...media] };
}
