import { expect, test } from "vitest";
import { getTextStats } from "./textStats";

test("counts English words, characters and lines", () => {
  expect(getTextStats("hello world\nsecond line")).toEqual({ characters: 23, words: 4, lines: 2 });
});

test("splits Chinese into words rather than one long token", () => {
  const { words, characters } = getTextStats("状态检查调度器");
  expect(characters).toBe(7);
  expect(words).toBeGreaterThan(1);
});

test("emoji counts as one character and punctuation is not a word", () => {
  expect(getTextStats("ok 👍!")).toMatchObject({ characters: 5, words: 1 });
});

test("empty text", () => {
  expect(getTextStats("")).toEqual({ characters: 0, words: 0, lines: 0 });
});
