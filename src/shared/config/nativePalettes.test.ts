import { expect, test } from "vitest";
import { NATIVE_PALETTES, normalizeNativePalette, getNativePaletteStyle, colorContrast, getColorForeground, colorToHsl, hslToColor } from "./nativePalettes";

test("keeps supported choices and rejects malformed custom colors", () => {
  for (const palette of NATIVE_PALETTES) expect(normalizeNativePalette(palette.id)).toBe(palette.id);
  expect(normalizeNativePalette("custom:#12AbEf")).toBe("custom:#12abef");
  for (const value of ["custom:red", "custom:#fff", "custom:#abcdef;display:none", "unknown"]) {
    expect(normalizeNativePalette(value)).toBe("default");
    expect(getNativePaletteStyle(value)).toEqual({});
  }
});

test("arbitrary colors keep text and filled controls readable in both modes", () => {
  for (const palette of ["mint", "lavender", "amber", ...["#000000", "#ffffff", "#ff0000", "#00ff00", "#0000ff", "#ffff00", "#fafafa", "#123456"].map(c => `custom:${c}`)]) {
    const style = getNativePaletteStyle(palette);
    for (const mode of ["light", "dark"]) {
      const key = `--native-palette-${mode}-`;
      const panel = `#${style[key + "panel"].split(", ").map(n => Number(n).toString(16).padStart(2,"0")).join("")}`;
      for (const role of ["accent", "text", "secondary", "muted"]) {
        expect(colorContrast(style[key + role], panel), `${palette} ${mode} ${role}`).toBeGreaterThanOrEqual(4.5);
      }
      expect(colorContrast(style[key + "filled-text"], style[key + "accent"])).toBeGreaterThanOrEqual(4.5);
    }
  }
  expect(getColorForeground("#ffffff")).toBe("#000000");
  expect(getColorForeground("#000000")).toBe("#ffffff");
});


test("picker colors round-trip without losing RGB values", () => {
  for (const color of ["#000000", "#ffffff", "#ff0000", "#00ff00", "#0000ff", "#123456", "#8655bb", "#abcdef"]) {
    expect(hslToColor(...colorToHsl(color))).toBe(color);
  }
  expect(hslToColor(360, 100, 50)).toBe("#ff0000");
});
