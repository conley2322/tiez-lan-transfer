import { describe, expect, it } from "vitest";
import { translations } from "./locales";

describe("i18n translations consistency", () => {
  const languages = ["zh", "en", "tw"] as const;

  it("provides all mandatory top-level language packs", () => {
    for (const lang of languages) {
      expect(translations[lang]).toBeDefined();
      expect(typeof translations[lang]).toBe("object");
    }
  });

  it("maintains key parity across zh, en, and tw", () => {
    const zhKeys = Object.keys(translations.zh).sort();
    const enKeys = Object.keys(translations.en).sort();
    const twKeys = Object.keys(translations.tw).sort();

    expect(zhKeys.length).toBeGreaterThan(500);

    const missingInEn = zhKeys.filter((key) => !Object.prototype.hasOwnProperty.call(translations.en, key));
    const missingInTw = zhKeys.filter((key) => !Object.prototype.hasOwnProperty.call(translations.tw, key));

    expect(missingInEn).toEqual([]);
    expect(missingInTw).toEqual([]);

    const extraInEn = enKeys.filter((key) => !Object.prototype.hasOwnProperty.call(translations.zh, key));
    const extraInTw = twKeys.filter((key) => !Object.prototype.hasOwnProperty.call(translations.zh, key));

    expect(extraInEn).toEqual([]);
    expect(extraInTw).toEqual([]);
  });

  it("contains non-empty translated strings for critical UI controls", () => {
    const criticalKeys: Array<keyof (typeof translations)["zh"]> = [
      "app_name",
      "settings",
      "search_placeholder",
      "clear_history",
      "sequential_paste_mode",
      "cloud_sync_settings",
      "settings_section_theme",
      "visual_theme",
    ];

    for (const key of criticalKeys) {
      for (const lang of languages) {
        const val = translations[lang][key];
        expect(val).toBeDefined();
        expect(typeof val).toBe("string");
        expect(val.trim().length).toBeGreaterThan(0);
      }
    }
  });
});
