import { afterEach, expect, test, vi } from "vitest";
import {
  getSelectableThemes, migrateThemeSettings, resolveThemeAppearance,
  applyNativePalette, applyThemeClasses, getAppliedNativePalette
} from "./themes";

afterEach(() => vi.unstubAllGlobals());

for (const [platform, nativeTheme] of [["MacIntel", "macos"], ["Win32", "mica"]]) {
  test(`${platform}: migrates Sakura without changing other settings`, () => {
    vi.stubGlobal("navigator", { platform, userAgent: platform });
    const before = { "app.theme": "sakura", "app.color_mode": "dark", "app.custom_background": "/image.png", "app.surface_opacity": "30" };
    const migrated = migrateThemeSettings(before);
    expect(migrated).toEqual({ ...before, "app.theme": nativeTheme, "app.native_palette": "sakura" });
    expect(before["app.theme"]).toBe("sakura");
    expect(migrateThemeSettings(migrated)).toBe(migrated);
    expect(resolveThemeAppearance("sakura")).toEqual({ theme: nativeTheme, nativePalette: "sakura" });
    expect(getSelectableThemes().map(t => t.id)).toContain(nativeTheme);
    expect(getSelectableThemes().map(t => t.id)).not.toContain("sakura");
  });
}

test("leaves existing native, other and installed themes intact", () => {
  for (const theme of ["macos", "mica", "paper", "store-custom"]) {
    const settings = { "app.theme": theme, "app.native_palette": "sakura" };
    expect(migrateThemeSettings(settings)).toBe(settings);
  }
  expect(resolveThemeAppearance("store-custom", "sakura").theme).toBe("store-custom");
  expect(resolveThemeAppearance("paper", "unknown").nativePalette).toBe("default");
});

test("switching palettes and themes clears retired classes without affecting dark mode", () => {
  vi.stubGlobal("navigator", { platform: "MacIntel", userAgent: "Mac" });
  const classes = new Set(["theme-sakura", "dark-mode"]);
  const target = { setAttribute: () => {}, classList: {
    add: (...names: string[]) => names.forEach(n => classes.add(n)),
    remove: (...names: string[]) => names.forEach(n => classes.delete(n)),
    toggle: (name: string, force: boolean) => force ? classes.add(name) : classes.delete(name),
    [Symbol.iterator]: () => classes.values()
  } } as unknown as Element;
  applyThemeClasses("sakura", target);
  applyNativePalette("sakura", undefined, target);
  expect([...classes].sort()).toEqual(["dark-mode", "native-palette-sakura", "theme-macos"]);
  applyNativePalette("macos", "default", target);
  expect(classes.has("native-palette-sakura")).toBe(false);
  applyNativePalette("macos", "sakura", target);
  applyThemeClasses("paper", target);
  applyNativePalette("paper", "sakura", target);
  expect([...classes].sort()).toEqual(["dark-mode", "theme-paper"]);
});


test("custom palette styles are cleared on switching away and preview data stays exact", () => {
  vi.stubGlobal("navigator", { platform: "MacIntel", userAgent: "Mac" });
  const classes = new Set<string>(), styles = new Map<string, string>([["--other-setting", "42"]]);
  const attributes = new Map<string, string>();
  const target = {
    classList: { toggle: (name: string, force: boolean) => force ? classes.add(name) : classes.delete(name), contains: (name: string) => classes.has(name) },
    setAttribute: (name: string, value: string) => attributes.set(name, value),
    getAttribute: (name: string) => attributes.get(name),
    style: { get length() { return styles.size; }, item: (index: number) => [...styles.keys()][index],
      setProperty: (name: string, value: string) => styles.set(name, value), removeProperty: (name: string) => styles.delete(name) }
  } as unknown as Element;
  applyNativePalette("macos", "custom:#12abef", target);
  expect(getAppliedNativePalette(target)).toBe("custom:#12abef");
  expect(classes.has("native-palette-color")).toBe(true);
  expect(styles.has("--native-palette-dark-accent")).toBe(true);
  applyNativePalette("macos", "sakura", target);
  expect(classes.has("native-palette-color")).toBe(false);
  expect([...styles]).toEqual([["--other-setting", "42"]]);
  applyNativePalette("paper", "custom:#12abef", target);
  expect(getAppliedNativePalette(target)).toBe("default");
  expect([...styles]).toEqual([["--other-setting", "42"]]);
});
