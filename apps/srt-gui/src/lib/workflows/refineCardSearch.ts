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

/** Builds reusable card text/index data and applies the Refine tab's search semantics. */
export function createRefineCardSearch<Card extends SearchableRefineCard>() {
  const htmlStripCache = new Map<string, string>();

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
        cards.map((card) => [card.id, {
          expression: stripHtml(card.expression),
          meaning: stripHtml(card.meaning),
        }]),
      );
      // Retain only the current dataset: switching decks must release old text,
      // while edits to the same deck can reuse all unchanged entries.
      const currentTexts = new Set(cards.flatMap(card => [card.expression, card.meaning]));
      for (const text of htmlStripCache.keys()) {
        if (!currentTexts.has(text)) htmlStripCache.delete(text);
      }
      const indices = new Map(cards.map((card, index) => [card.id, index]));
      return { plainTexts, indices };
    },

    filter(cards: Card[], plainTexts: Map<string, { expression: string; meaning: string }>, searchQuery: string): Card[] {
      const normalizedSearch = searchQuery.trim().toLowerCase();
      const filteredCards = normalizedSearch
        ? cards.filter((card) => {
            const plain = plainTexts.get(card.id);
            const expression = plain ? plain.expression.toLowerCase() : card.expression.toLowerCase();
            const meaning = plain ? plain.meaning.toLowerCase() : card.meaning.toLowerCase();
            return expression.includes(normalizedSearch)
              || meaning.includes(normalizedSearch)
              || card.notes.toLowerCase().includes(normalizedSearch);
          })
        : cards;

      return filteredCards;
    },
  };
}
