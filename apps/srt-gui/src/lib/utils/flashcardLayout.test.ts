import { expect, it } from "vitest";
import { computeEffectiveColumnCount } from "./flashcardLayout";
it("chooses columns from usable workspace width, including narrow windows", () => {
  expect([340, 899, 900, 1199, 1200, 1800].map(computeEffectiveColumnCount)).toEqual([1, 1, 2, 2, 3, 3]);
});
