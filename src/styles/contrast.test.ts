// Resolves each built-in theme's color tokens in light and dark mode straight
// from the stylesheets, then checks the text roles against the surfaces they
// sit on (docs/DESIGN-SYSTEM.md, "Color roles").
import { describe, expect, test } from "vitest";
import { readFileSync, readdirSync } from "node:fs";

// Vitest turns CSS imports (even ?raw) into empty strings, so read the files.
const dir = new URL(".", import.meta.url);
const read = (name: string) => readFileSync(new URL(name, dir), "utf8");
const baseCss = read("base.css");
const themeCss = Object.fromEntries(readdirSync(new URL("themes/", dir)).filter((f) => f.endsWith(".css")).map((f) => [f, read(`themes/${f}`)]));

const THEMES = ["mica", "macos", "acrylic", "retro", "sticky-note", "paper", "terminal", "receipt", "ink"];
const MINIMUM: Record<string, number> = { "--text-primary": 7, "--text-secondary": 5.5, "--text-muted": 4.5 };

type Rule = { selector: string; decls: Map<string, string>; order: number; media: string | null };
type RGBA = [number, number, number, number];

// ---- Parsing: custom-property declarations only, with their selector and @media.
const parse = (css: string, orderBase: number): Rule[] => {
  const text = css.replace(/\/\*[\s\S]*?\*\//g, "");
  const rules: Rule[] = [];
  const walk = (src: string, media: string | null) => {
    let i = 0;
    while (i < src.length) {
      const open = src.indexOf("{", i);
      if (open === -1) break;
      const head = src.slice(i, open).trim();
      let depth = 1, j = open + 1;
      while (j < src.length && depth) { if (src[j] === "{") depth++; else if (src[j] === "}") depth--; j++; }
      const body = src.slice(open + 1, j - 1);
      if (head.startsWith("@media")) walk(body, head);
      else if (!head.startsWith("@")) {
        const decls = new Map<string, string>();
        for (const m of body.matchAll(/(--[\w-]+)\s*:\s*([^;]+);?/g)) decls.set(m[1], m[2].replace(/!important/, "").trim());
        if (decls.size) rules.push({ selector: head, decls, order: orderBase + rules.length, media });
      }
      i = j;
    }
  };
  walk(text, null);
  return rules;
};

// ---- Selector matching for <html> and <body> only.
type El = { tag: "html" | "body"; classes: Set<string> };
const splitTop = (s: string, sep: string) => {
  const out: string[] = []; let depth = 0, cur = "";
  for (const ch of s) {
    if (ch === "(") depth++; else if (ch === ")") depth--;
    if (ch === sep && depth === 0) { out.push(cur); cur = ""; } else cur += ch;
  }
  out.push(cur);
  return out.map((x) => x.trim()).filter(Boolean);
};
/** Returns the specificity [classes, tags] if the compound matches, else null. */
const matchCompound = (compound: string, el: El): [number, number] | null => {
  let rest = compound, cls = 0, tags = 0;
  const tag = rest.match(/^(html|body|\*)/);
  if (tag) { if (tag[1] !== "*" && tag[1] !== el.tag) return null; if (tag[1] !== "*") tags++; rest = rest.slice(tag[1].length); }
  while (rest) {
    let m: RegExpMatchArray | null;
    if ((m = rest.match(/^\.([\w-]+)/))) { if (!el.classes.has(m[1])) return null; cls++; }
    else if ((m = rest.match(/^:root/))) { if (el.tag !== "html") return null; cls++; }
    else if ((m = rest.match(/^:(not|is|where)\(/))) {
      let depth = 1, k = m[0].length;
      while (k < rest.length && depth) { if (rest[k] === "(") depth++; else if (rest[k] === ")") depth--; k++; }
      const args = splitTop(rest.slice(m[0].length, k - 1), ",");
      const hits = args.map((a) => matchCompound(a, el));
      const best = Math.max(0, ...args.map((a) => (a.match(/\./g) || []).length + (a.includes(":root") ? 1 : 0)));
      if (m[1] === "not") { if (hits.some(Boolean)) return null; cls += best; }
      else { if (!hits.some(Boolean)) return null; if (m[1] === "is") cls += best; }
      m = [rest.slice(0, k)];
    } else return null; // attributes, pseudo-elements, other pseudo-classes
    rest = rest.slice(m[0].length);
  }
  return [cls, tags];
};
const matchSelector = (selector: string, el: El, html: El): [number, number] | null => {
  const parts = selector.trim().split(/\s*>\s*|\s+/).filter(Boolean);
  const last = matchCompound(parts[parts.length - 1], el);
  if (!last) return null;
  if (parts.length === 1) return last;
  if (parts.length > 2 || el.tag !== "body") return null;
  const anc = matchCompound(parts[0], html);
  return anc ? [last[0] + anc[0], last[1] + anc[1]] : null;
};

const computeTokens = (rules: Rule[], theme: string, mode: "light" | "dark") => {
  const classes = new Set([`theme-${theme}`, `${mode}-mode`]);
  const html: El = { tag: "html", classes }, body: El = { tag: "body", classes };
  const apply = (el: El, inherited: Map<string, string>) => {
    const hits: { spec: [number, number]; order: number; decls: Map<string, string> }[] = [];
    for (const r of rules) {
      if (r.media && !(r.media.includes("prefers-color-scheme: dark") && mode === "dark")) continue;
      for (const sel of splitTop(r.selector, ",")) {
        const spec = matchSelector(sel, el, html);
        if (spec) hits.push({ spec, order: r.order, decls: r.decls });
      }
    }
    hits.sort((a, b) => a.spec[0] - b.spec[0] || a.spec[1] - b.spec[1] || a.order - b.order);
    const out = new Map(inherited);
    for (const h of hits) for (const [k, v] of h.decls) out.set(k, v);
    return out;
  };
  return apply(body, apply(html, new Map()));
};

// ---- Values: var() resolution and colors.
const resolve = (value: string, tokens: Map<string, string>, depth = 0): string => {
  if (depth > 20) return value;
  return value.replace(/var\(\s*(--[\w-]+)\s*(?:,((?:[^()]|\([^()]*(?:\([^()]*\))*[^()]*\))*))?\)/g, (_, name: string, fallback?: string) => {
    const v = tokens.get(name);
    return v !== undefined ? resolve(v, tokens, depth + 1) : fallback !== undefined ? resolve(fallback.trim(), tokens, depth + 1) : "";
  });
};
/** Folds calc(), clamp(), min() and max() over plain numbers. */
const evalMath = (input: string): string => {
  let s = input, prev = "";
  while (s !== prev) {
    prev = s;
    s = s.replace(/\b(calc|clamp|min|max)\(([^()]*)\)/g, (whole, fn: string, args: string) => {
      const nums = splitTop(args, ",").map((a) => (/^[\d.\s+\-*/]+$/.test(a) ? Number(Function(`return (${a})`)()) : NaN));
      if (nums.some(Number.isNaN)) return whole;
      const v = fn === "calc" ? nums[0] : fn === "min" ? Math.min(...nums) : fn === "max" ? Math.max(...nums) : Math.min(Math.max(nums[1], nums[0]), nums[2]);
      return String(v);
    });
  }
  return s;
};
const NAMED: Record<string, RGBA> = { white: [255, 255, 255, 1], black: [0, 0, 0, 1], transparent: [0, 0, 0, 0] };
const parseColor = (raw: string): RGBA | null => {
  const s = raw.trim();
  if (NAMED[s]) return NAMED[s];
  let m = s.match(/^#([0-9a-f]{3,8})$/i);
  if (m) {
    const h = m[1].length <= 4 ? [...m[1]].map((c) => c + c).join("") : m[1];
    return [parseInt(h.slice(0, 2), 16), parseInt(h.slice(2, 4), 16), parseInt(h.slice(4, 6), 16), h.length === 8 ? parseInt(h.slice(6, 8), 16) / 255 : 1];
  }
  if ((m = s.match(/^rgba?\((.*)\)$/))) {
    const p = evalMath(m[1]).split(/[\s,/]+/).filter(Boolean).map(Number);
    return [p[0], p[1], p[2], p.length > 3 ? p[3] : 1];
  }
  if ((m = s.match(/^color-mix\(\s*in srgb\s*,(.*)\)$/))) {
    const [a, b] = splitTop(m[1], ",").map((part) => {
      const pm = part.match(/^(.*?)\s+([\d.]+)%$/);
      return { color: parseColor(pm ? pm[1] : part), pct: pm ? Number(pm[2]) / 100 : null };
    });
    if (!a?.color || !b?.color) return null;
    let p1 = a.pct, p2 = b.pct;
    if (p1 === null && p2 === null) p1 = p2 = 0.5;
    else if (p1 === null) p1 = 1 - (p2 as number);
    else if (p2 === null) p2 = 1 - p1;
    const sum = (p1 as number) + (p2 as number);
    const w1 = (p1 as number) / sum, w2 = (p2 as number) / sum;
    const alpha = a.color[3] * w1 + b.color[3] * w2;
    const ch = (k: number) => (alpha ? (a.color![k] * a.color![3] * w1 + b.color![k] * b.color![3] * w2) / alpha : 0);
    return [ch(0), ch(1), ch(2), alpha * Math.min(sum, 1)];
  }
  if ((m = s.match(/^(?:linear|radial)-gradient\((.*)\)$/))) {
    const stops = splitTop(m[1], ",").map((part) => parseColor(part.replace(/\s+[\d.]+%(\s+[\d.]+%)?$/, ""))).filter(Boolean) as RGBA[];
    if (!stops.length) return null;
    return stops.reduce((acc, c) => acc.map((v, k) => v + c[k] / stops.length) as RGBA, [0, 0, 0, 0] as RGBA);
  }
  return null;
};
const over = (top: RGBA, bottom: RGBA): RGBA => {
  const a = top[3] + bottom[3] * (1 - top[3]);
  if (!a) return [0, 0, 0, 0];
  const ch = (k: number) => (top[k] * top[3] + bottom[k] * bottom[3] * (1 - top[3])) / a;
  return [ch(0), ch(1), ch(2), a];
};
const luminance = (c: RGBA) => [0, 1, 2].reduce((sum, k) => {
  const v = c[k] / 255;
  return sum + (v <= 0.04045 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4) * [0.2126, 0.7152, 0.0722][k];
}, 0);
const contrast = (a: RGBA, b: RGBA) => {
  const [hi, lo] = [luminance(a), luminance(b)].sort((x, y) => y - x);
  return (hi + 0.05) / (lo + 0.05);
};

const rules = [
  ...parse(baseCss, 0),
  ...Object.entries(themeCss).sort(([a], [b]) => a.localeCompare(b)).flatMap(([, css], i) => parse(css, (i + 1) * 100000)),
];

describe("text contrast per theme", () => {
  for (const theme of THEMES) for (const mode of ["light", "dark"] as const) {
    test(`${theme} ${mode}`, () => {
      const tokens = computeTokens(rules, theme, mode);
      const color = (name: string) => parseColor(resolve(tokens.get(name) ?? "", tokens));
      // Windows behind translucent themes are unknown; assume a mid backdrop of the mode.
      const backdrop: RGBA = mode === "dark" ? [20, 20, 20, 1] : [245, 245, 245, 1];
      const layer = (name: string, under: RGBA) => over(color(name) ?? [0, 0, 0, 0], under);
      const body = layer("--bg-body", backdrop);
      const content = layer("--bg-content", layer("--bg-window", body));
      const surfaces = {
        content,
        element: over(color("--layout-item-background") ?? color("--bg-element") ?? [0, 0, 0, 0], content),
        panel: layer("--bg-panel", content),
        input: layer("--bg-input", content),
      };
      const failures: string[] = [];
      for (const [role, min] of Object.entries(MINIMUM)) {
        const fg = color(role);
        expect(fg, `${role} must resolve to a color`).not.toBeNull();
        for (const [surface, bg] of Object.entries(surfaces)) {
          const ratio = contrast(over(fg!, bg), bg);
          if (ratio < min) failures.push(`${role} on ${surface}: ${ratio.toFixed(2)} < ${min}`);
        }
      }
      expect(failures).toEqual([]);
    });
  }
});
