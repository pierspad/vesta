import { describe, expect, it } from "vitest";
import { createRefineNotesHistory } from "./refineNotesHistory";

describe("refine notes history", () => {
  it("captures notes only and restores cards by id", () => {
    let cards = [
      { id: "a", notes: "before A", initialNotes: "original A", expression: "first" },
      { id: "b", notes: "before B", initialNotes: "original B", expression: "second" },
    ];
    const history = createRefineNotesHistory(
      () => cards,
      (updates) => { cards = cards.map(card => ({ ...card, notes: updates.find(entry => entry.id === card.id)?.notes ?? card.notes })); },
    );

    history.push();
    cards = cards.map(card => ({ ...card, notes: `after ${card.id}` }));
    expect(history.undo()).toBe(true);
    expect(cards.map(card => [card.id, card.notes])).toEqual([["a", "before A"], ["b", "before B"]]);
    expect(cards.map(card => card.initialNotes)).toEqual(["original A", "original B"]);
    expect(history.redo()).toBe(true);
    expect(cards.map(card => [card.id, card.notes])).toEqual([["a", "after a"], ["b", "after b"]]);
    history.dispose();
  });

  it("does not restore notes onto a different document after history reset", () => {
    let cards = [{ id: "old", notes: "old note" }];
    const history = createRefineNotesHistory(() => cards, updates => { cards = cards.map(card => ({ ...card, notes: updates.find(entry => entry.id === card.id)?.notes ?? card.notes })); });
    history.push();
    cards = [{ id: "new", notes: "new note" }];
    history.reset();
    expect(history.undo()).toBe(false);
    expect(cards).toEqual([{ id: "new", notes: "new note" }]);
    history.dispose();
  });
});
