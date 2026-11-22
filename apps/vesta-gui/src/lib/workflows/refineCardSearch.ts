export interface SearchableRefineCard {
  id: string;
  expression: string;
  meaning: string;
  notes: string;
}

export interface RefineCardSearchIndex<Card extends SearchableRefineCard> {
  plainTexts: Map<string, { expression: string; meaning: string }>;
  indices: Map<string, number>;
}

interface CardSearchCacheEntry {
  expressionLower: string;
  meaningLower: string;
  lastNotes: string;
  notesLower: string;
}

/** Builds reusable card text/index data and applies the Refine tab's search semantics. */
export function createRefineCardSearch<Card extends SearchableRefineCard>() {
  const htmlStripCache = new Map<string, string>();
  const cardSearchCache = new Map<string, CardSearchCacheEntry>();

  function stripHtml(text: string): string {
    if (!text) return "";
    let stripped = htmlStripCache.get(text);
    if (stripped === undefined) {
      stripped = text.replace(/<[^>]*>/g, "");
      htmlStripCache.set(text, stripped);
    }
    return stripped;
  }

  return {
    index(cards: Card[]): RefineCardSearchIndex<Card> {
      const plainTexts = new Map(
        cards.map((card) => {
          const expression = stripHtml(card.expression);
          const meaning = stripHtml(card.meaning);
          const notes = card.notes ?? "";
          cardSearchCache.set(card.id, {
            expressionLower: expression.toLowerCase(),
            meaningLower: meaning.toLowerCase(),
            lastNotes: notes,
            notesLower: notes.toLowerCase(),
          });
          return [card.id, { expression, meaning }];
        }),
      );
      // Retain only the current dataset: switching decks must release old text,
      // while edits to the same deck can reuse all unchanged entries.
      const currentTexts = new Set(cards.flatMap(card => [card.expression, card.meaning]));
      for (const text of htmlStripCache.keys()) {
        if (!currentTexts.has(text)) htmlStripCache.delete(text);
      }
      const currentIds = new Set(cards.map(card => card.id));
      for (const id of cardSearchCache.keys()) {
        if (!currentIds.has(id)) cardSearchCache.delete(id);
      }
      const indices = new Map(cards.map((card, index) => [card.id, index]));
      return { plainTexts, indices };
    },

    filter(cards: Card[], plainTexts: Map<string, { expression: string; meaning: string }>, searchQuery: string): Card[] {
      const normalizedSearch = searchQuery.trim().toLowerCase();
      if (!normalizedSearch) return cards;

      return cards.filter((card) => {
        let cached = cardSearchCache.get(card.id);
        if (!cached) {
          const plain = plainTexts.get(card.id);
          const expression = plain ? plain.expression : stripHtml(card.expression);
          const meaning = plain ? plain.meaning : stripHtml(card.meaning);
          const notes = card.notes ?? "";
          cached = {
            expressionLower: expression.toLowerCase(),
            meaningLower: meaning.toLowerCase(),
            lastNotes: notes,
            notesLower: notes.toLowerCase(),
          };
          cardSearchCache.set(card.id, cached);
        } else if (card.notes !== cached.lastNotes) {
          cached.lastNotes = card.notes ?? "";
          cached.notesLower = cached.lastNotes.toLowerCase();
        }

        return (
          cached.expressionLower.includes(normalizedSearch) ||
          cached.meaningLower.includes(normalizedSearch) ||
          cached.notesLower.includes(normalizedSearch)
        );
      });
    },
  };
}
