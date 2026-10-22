import { describe, expect, it } from "vitest";
import { EMOJI_NAMES } from "./emojiNames";

describe("Emoji Names Catalog", () => {
  it("contains valid trilingual names (zh, zh-TW, en) for emojis", () => {
    const emojis = Object.keys(EMOJI_NAMES);
    expect(emojis.length).toBeGreaterThan(100);

    for (const char of emojis.slice(0, 30)) {
      const entry = EMOJI_NAMES[char];
      expect(entry.zh).toBeDefined();
      expect(entry["zh-TW"]).toBeDefined();
      expect(entry.en).toBeDefined();
      expect(entry.zh.length).toBeGreaterThan(0);
      expect(entry.en.length).toBeGreaterThan(0);
    }
  });

  it("translates popular emojis properly", () => {
    expect(EMOJI_NAMES["😀"].zh).toBe("大笑");
    expect(EMOJI_NAMES["😀"].en).toBe("Grinning Face");

    expect(EMOJI_NAMES["😂"].zh).toBe("笑哭");
    expect(EMOJI_NAMES["😂"].en).toBe("Tears of Joy");
  });
});
