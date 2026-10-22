import { expect, it } from "vitest";
import { linkedDimensions } from "./aspectRatio";
it("links both dimensions with a ratio captured independently of rounded outputs", () => {
  expect(linkedDimensions(16/9, "width", 1280)).toEqual({width:1280,height:720});
  expect(linkedDimensions(4/3, "height", 480)).toEqual({width:640,height:480});
  expect(linkedDimensions(16/9, "height", 10000)).toEqual({width:7680,height:4320});
  expect(linkedDimensions(16/9, "width", 1)).toEqual({width:28,height:16});
});
it("ignores empty or invalid dimensions", () => {
  expect(linkedDimensions(0, "width", 640)).toBeNull();
  expect(linkedDimensions(16/9, "width", NaN)).toBeNull();
});
