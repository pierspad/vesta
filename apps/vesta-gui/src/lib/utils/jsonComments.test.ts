import { expect, it } from "vitest";
import { parseJsonWithComments } from "./jsonComments";
it("preserves escaped backslashes, quotes and comment markers inside JSON strings", () => {
  const value = { path: "C:\\", quote: 'say "hi" // literal', markers: "https://a/*b*/" };
  const json = JSON.stringify(value);
  expect(parseJsonWithComments(`/* leading */ ${json} // trailing`)).toEqual(value);
});
it("keeps comments from joining invalid JSON tokens and preserves line breaks", () => {
  expect(() => parseJsonWithComments('{"value": 1/*comment*/2}')).toThrow();
  expect(parseJsonWithComments('{\n//line\n"value": true /* block\ncomment */}')).toEqual({ value: true });
});
it("rejects unterminated strings and comments", () => {
  for (const json of ['{"a":"broken}', '{"a": 1 /* broken }']) expect(() => parseJsonWithComments(json)).toThrow();
});
