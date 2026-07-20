/** Card-length/duration filter settings for flashcard generation. Grouped
 * into one reactive object so CardFiltersPanel.svelte can own the panel UI
 * via a single `bind:filters` prop instead of a dozen loose bindings — mirrors
 * the mediaSettings/EpisodeMediaOverrides shape in flashcardMediaTypes.ts. */
export interface CardFilterSettings {
  enabled: boolean;
  minChars: number;
  maxChars: number;
  minCharsEnabled: boolean;
  maxCharsEnabled: boolean;
  minDurationMs: number;
  maxDurationMs: number;
  minDurationEnabled: boolean;
  maxDurationEnabled: boolean;
  combineSentences: boolean;
  continuationChars: string;
}
