import { describe, expect, test } from "vitest";
import { hotkeyKeyFromEvent, isTypingHotkey } from "./hotkeyKey";
import { matchesHotkey } from "../hooks/useHotkeyMatching";

const ev = (code: string, key: string, mods: Partial<KeyboardEvent> = {}) =>
  ({ code, key, ctrlKey: false, shiftKey: false, altKey: false, metaKey: false, ...mods }) as KeyboardEvent;

describe("hotkeyKeyFromEvent", () => {
  test("uses the physical key, not the character Option or Shift types", () => {
    expect(hotkeyKeyFromEvent(ev("KeyC", "ç", { altKey: true }))).toBe("C");
    expect(hotkeyKeyFromEvent(ev("Digit1", "!", { shiftKey: true }))).toBe("1");
    expect(hotkeyKeyFromEvent(ev("Space", " "))).toBe("Space");
    expect(hotkeyKeyFromEvent(ev("ArrowUp", "ArrowUp"))).toBe("Up");
    expect(hotkeyKeyFromEvent(ev("Comma", "≤", { altKey: true }))).toBe("Comma");
    expect(hotkeyKeyFromEvent(ev("F5", "F5"))).toBe("F5");
  });

  test("ignores bare modifiers", () => {
    for (const code of ["AltLeft", "ShiftRight", "MetaLeft", "ControlLeft"]) {
      expect(hotkeyKeyFromEvent(ev(code, "Alt"))).toBeNull();
    }
  });
});

describe("matchesHotkey with Option", () => {
  test("Alt+F matches Option+F on macOS, which types ƒ", () => {
    expect(matchesHotkey(ev("KeyF", "ƒ", { altKey: true }), "Alt+F")).toBe(true);
  });
  test("Shift+1 matches although the key is !", () => {
    expect(matchesHotkey(ev("Digit1", "!", { shiftKey: true }), "Shift+1")).toBe(true);
  });
  test("a different key still does not match", () => {
    expect(matchesHotkey(ev("KeyG", "©", { altKey: true }), "Alt+F")).toBe(false);
  });
});

describe("isTypingHotkey", () => {
  test("refuses shortcuts that would swallow typed characters", () => {
    for (const h of ["Shift+S", "Shift+1", "C", "Space", "Shift+Comma"]) expect(isTypingHotkey(h)).toBe(true);
  });
  test("allows real modifiers, function keys and clearing", () => {
    for (const h of ["Alt+C", "Command+Shift+V", "Ctrl+1", "F5", "Shift+F5", "MouseMiddle", ""]) {
      expect(isTypingHotkey(h)).toBe(false);
    }
  });
});
