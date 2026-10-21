import { describe, expect, it } from "vitest";
import { createRefineCardSearch } from "./refineCardSearch";

const cards = [
  { id: "one", expression: "<b>Hola</b>", meaning: "Hello", notes: "greeting" },
  { id: "two", expression: "ありがとう", meaning: "Thank you", notes: "POLITE" },
];

describe("createRefineCardSearch", () => {
  it("indexes source positions and strips markup for matching while retaining original cards", () => {
    const search = createRefineCardSearch<typeof cards[number]>();
    const result = search.index(cards);
    const filtered = search.filter(cards, result.plainTexts, "  HOLA ");

    expect(result.indices).toEqual(new Map([["one", 0], ["two", 1]]));
    expect(result.plainTexts.get("one")).toEqual({ expression: "Hola", meaning: "Hello" });
    expect(filtered).toEqual([cards[0]]);
    expect(filtered[0]).toBe(cards[0]);
  });

  it("searches notes case-insensitively and returns all original cards for an empty query", () => {
    const search = createRefineCardSearch<typeof cards[number]>();
    const index = search.index(cards);
    expect(search.filter(cards, index.plainTexts, " polite ")).toEqual([cards[1]]);
    expect(search.filter(cards, index.plainTexts, "  ")).toBe(cards);
  });

  it("caches stripped text across successive indexes", () => {
    const search = createRefineCardSearch<typeof cards[number]>();
    const first = search.index(cards);
    const second = search.index(cards);
    expect(second.plainTexts.get("one")).toEqual(first.plainTexts.get("one"));
  });
});
