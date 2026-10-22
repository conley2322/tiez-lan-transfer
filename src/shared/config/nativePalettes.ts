export const NATIVE_PALETTES = [
  { id: "default", labelKey: "native_palette_default", color: "#007aff" },
  { id: "sakura", labelKey: "theme_name_sakura", color: "#c64e6b" },
  { id: "mint", labelKey: "native_palette_mint", color: "#238368" },
  { id: "lavender", labelKey: "native_palette_lavender", color: "#8655bb" },
  { id: "amber", labelKey: "native_palette_amber", color: "#b66f16" }
];

export const normalizeCustomColor = (color?: string): string | undefined =>
  /^#[\da-f]{6}$/i.test(color || "") ? color!.toLowerCase() : undefined;

export const getCustomPaletteColor = (palette?: string): string | undefined =>
  palette?.startsWith("custom:") ? normalizeCustomColor(palette.slice(7)) : undefined;

export const normalizeNativePalette = (palette?: string): string => {
  const custom = getCustomPaletteColor(palette);
  if (custom) return `custom:${custom}`;
  return NATIVE_PALETTES.some(item => item.id === palette) ? palette! : "default";
};

export const getNativePaletteColor = (palette?: string): string | undefined =>
  getCustomPaletteColor(palette) || NATIVE_PALETTES.find(item => item.id === palette)?.color;

type RGB = [number, number, number];
const rgb = (hex: string): RGB => [1, 3, 5].map(i => parseInt(hex.slice(i, i + 2), 16)) as RGB;
const mix = (a: RGB, b: RGB, amount: number): RGB => a.map((v, i) => Math.round(v * (1 - amount) + b[i] * amount)) as RGB;
const hex = (color: RGB): string => `#${color.map(v => v.toString(16).padStart(2, "0")).join("")}`;
const luminance = (color: RGB): number => color.map(v => {
  const n = v / 255;
  return n <= 0.04045 ? n / 12.92 : ((n + 0.055) / 1.055) ** 2.4;
}).reduce((sum, v, i) => sum + v * [0.2126, 0.7152, 0.0722][i], 0);
export const colorContrast = (a: string, b: string): number => {
  const values = [luminance(rgb(a)), luminance(rgb(b))].sort((x, y) => x - y);
  return (values[1] + 0.05) / (values[0] + 0.05);
};
export const getColorForeground = (color: string): string =>
  colorContrast(color, "#ffffff") >= colorContrast(color, "#000000") ? "#ffffff" : "#000000";

// Preserve the chosen hue while making accent text readable on the tinted surface.
const readableAccent = (color: RGB, background: RGB, dark: boolean): RGB => {
  const endpoint: RGB = dark ? [255, 255, 255] : [0, 0, 0];
  for (let step = 0; step <= 50; step++) {
    const candidate = mix(color, endpoint, step / 50);
    if (colorContrast(hex(candidate), hex(background)) >= 4.5) return candidate;
  }
  return endpoint;
};

export const getNativePaletteStyle = (palette: string): Record<string, string> => {
  if (palette === "default" || palette === "sakura") return {};
  const selected = getNativePaletteColor(palette);
  if (!selected) return {};
  const color = rgb(selected);
  const styles: Record<string, string> = {};
  for (const dark of [false, true]) {
    const mode = dark ? "dark" : "light";
    const window = mix(dark ? [24, 24, 26] : [250, 250, 252], color, dark ? 0.08 : 0.04);
    const panel = mix(dark ? [35, 35, 38] : [255, 255, 255], color, dark ? 0.1 : 0.03);
    const background = dark
      ? (luminance(window) > luminance(panel) ? window : panel)
      : (luminance(window) < luminance(panel) ? window : panel);
    const accent = readableAccent(color, background, dark);
    const line = dark ? [235, 235, 240] : mix([90, 90, 95], color, 0.25);
    const tokens = {
      window: window.join(", "), panel: panel.join(", "),
      text: hex(readableAccent(mix(dark ? [242, 242, 247] : [29, 29, 31], color, 0.08), background, dark)),
      secondary: hex(readableAccent(mix(dark ? [192, 192, 200] : [100, 100, 108], color, 0.08), background, dark)),
      muted: hex(readableAccent(mix(dark ? [175, 175, 183] : [109, 109, 117], color, 0.08), background, dark)),
      accent: hex(accent), "accent-rgb": accent.join(", "),
      "accent-hover": hex(mix(accent, dark ? [255, 255, 255] : [0, 0, 0], 0.12)),
      line: `rgba(${line.join(", ")}, ${dark ? 0.1 : 0.15})`,
      "line-strong": `rgba(${line.join(", ")}, ${dark ? 0.18 : 0.24})`,
      input: dark ? "rgba(255, 255, 255, 0.06)" : "rgba(255, 255, 255, 0.9)",
      button: `rgba(${accent.join(", ")}, ${dark ? 0.09 : 0.07})`,
      "filled-text": getColorForeground(hex(accent))
    };
    for (const [name, value] of Object.entries(tokens)) styles[`--native-palette-${mode}-${name}`] = value;
  }
  return styles;
};

export const colorToHsl = (color: string): [number, number, number] => {
  const [r, g, b] = rgb(color).map(v => v / 255);
  const max = Math.max(r, g, b), min = Math.min(r, g, b), delta = max - min;
  const lightness = (max + min) / 2;
  if (!delta) return [0, 0, lightness * 100];
  const hue = max === r ? ((g - b) / delta + (g < b ? 6 : 0))
    : max === g ? (b - r) / delta + 2 : (r - g) / delta + 4;
  return [hue * 60, delta / (1 - Math.abs(2 * lightness - 1)) * 100, lightness * 100];
};

export const hslToColor = (hue: number, saturation: number, lightness: number): string => {
  const l = lightness / 100, s = saturation / 100;
  const chroma = (1 - Math.abs(2 * l - 1)) * s;
  const h = (hue % 360) / 60, x = chroma * (1 - Math.abs(h % 2 - 1));
  const parts = h < 1 ? [chroma, x, 0] : h < 2 ? [x, chroma, 0] : h < 3 ? [0, chroma, x]
    : h < 4 ? [0, x, chroma] : h < 5 ? [x, 0, chroma] : [chroma, 0, x];
  return hex(parts.map(v => Math.round((v + l - chroma / 2) * 255)) as RGB);
};
