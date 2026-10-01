export interface FileDiscoveryOptions {
  enabled: boolean;
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
  const subtitles = new Set(subtitleFiles);
  const media = new Set(mediaFiles);
  const suggestions = await discover([...media], async path => {
    try {
      return await options.invoke<{ target: string | null; native: string | null }>("sync_suggest_subtitles_for_media", {
        mediaPath: path, defaultTargetLang: options.targetLanguage || null, defaultNativeLang: options.nativeLanguage || null,
      });
    } catch { return null; }
  });
  for (const suggestion of suggestions) {
    if (suggestion?.target) subtitles.add(suggestion.target);
    if (suggestion?.native) subtitles.add(suggestion.native);
  }
  const companions = await discover([...subtitles], async path => {
    let subtitle: string | null = null;
    let mediaPath: string | null = null;
    try { subtitle = await options.invoke<string | null>("sync_suggest_companion_subtitle_for_srt", { srtPath: path }); } catch { /* best effort */ }
    try { mediaPath = await options.invoke<string | null>("sync_suggest_media_for_srt", { srtPath: path }); } catch { /* best effort */ }
    return { subtitle, mediaPath };
  });
  for (const companion of companions) {
    if (companion.subtitle) subtitles.add(companion.subtitle);
    if (companion.mediaPath) media.add(companion.mediaPath);
  }
  return { subtitleFiles: [...subtitles], mediaFiles: [...media] };
}
