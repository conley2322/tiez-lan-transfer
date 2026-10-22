#!/usr/bin/env node
// Enforces the rules in docs/DESIGN-SYSTEM.md on src/styles.
//
//   node scripts/lint-styles.mjs            fail if any file got worse than the baseline
//   node scripts/lint-styles.mjs --update   rewrite the baseline after fixing violations
//   node scripts/lint-styles.mjs --report   list every violation, baseline or not
//
// Existing violations are recorded per file and rule in style-lint-baseline.json;
// a count may only go down. A declaration the rules cannot express can be
// exempted with a trailing `/* design-ok: reason */` comment.
import { readFileSync, writeFileSync, existsSync, readdirSync, statSync } from "node:fs";
import { join, relative, sep } from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = fileURLToPath(new URL("..", import.meta.url));
const STYLES = join(ROOT, "src", "styles");
const BASELINE = join(ROOT, "scripts", "style-lint-baseline.json");

// Half-steps below 12 suit the compact window; beyond 24, any multiple of 4.
const SPACE_PX = new Set([0, 1, 2, 4, 6, 8, 10, 12, 16, 20, 24]);
const onSpaceScale = (n) => SPACE_PX.has(n) || (n >= 24 && n % 4 === 0);
const SPACING_PROPS = /^(padding|margin)(-(top|right|bottom|left|inline|block)(-(start|end))?)?$|^(gap|row-gap|column-gap)$/;
const COLOR_LITERAL = /#[0-9a-fA-F]{3,8}\b|\b(?:rgba?|hsla?)\(\s*[\d.]/;
const PALETTE_FILES = new Set(["base.css"]);
const THEME_SELECTOR = /\.theme-(mica|macos|acrylic|retro|sticky-note|paper|terminal|receipt|ink|sakura)\b/;

const walk = (dir) => readdirSync(dir).flatMap((name) => {
  const path = join(dir, name);
  return statSync(path).isDirectory() ? walk(path) : path.endsWith(".css") ? [path] : [];
});

const rel = (path) => relative(STYLES, path).split(sep).join("/");
const isTheme = (file) => file.startsWith("themes/");

/** Comments blanked out (newlines kept) so offsets still map to lines. */
const blankComments = (text) => text.replace(/\/\*[\s\S]*?\*\//g, (c) => c.replace(/[^\n]/g, " "));

const lineOf = (text, index) => text.slice(0, index).split("\n").length;

function* declarations(text) {
  const clean = blankComments(text);
  // Track the enclosing selector and at-rules with a brace stack.
  // Entries: [selector, exempt]. A design-ok comment in a rule's header
  // exempts every declaration of that rule.
  const stack = [];
  let start = 0;
  for (let i = 0; i < clean.length; i++) {
    const ch = clean[i];
    if (ch === "{") {
      stack.push([clean.slice(start, i).trim(), /design-ok/.test(text.slice(start, i))]);
      start = i + 1;
    } else if (ch === "}" || ch === ";") {
      const chunk = clean.slice(start, i);
      const colon = chunk.indexOf(":");
      if (colon > 0 && stack.length) {
        const prop = chunk.slice(0, colon).trim();
        if (/^(--)?[a-zA-Z0-9_-]+$/.test(prop)) {
          const valueStart = start + colon + 1;
          const raw = text.slice(start, i + 1);
          // A design-ok comment may sit after the semicolon on the same line.
          const lineEnd = text.indexOf("\n", i);
          const tail = text.slice(i, lineEnd === -1 ? undefined : lineEnd);
          yield {
            prop,
            value: clean.slice(valueStart, i).trim(),
            line: lineOf(text, start + chunk.search(/\S/)),
            selector: stack[stack.length - 1][0],
            context: stack.map((e) => e[0]).join(" › "),
            exempt: stack[stack.length - 1][1] || /design-ok/.test(raw) || /design-ok/.test(tail),
          };
        }
      }
      if (ch === "}") stack.pop();
      start = i + 1;
    }
  }
}

function lintFile(file, text) {
  const out = [];
  const seenThemeRule = new Set();
  const add = (rule, d, message) => { if (!d.exempt) out.push({ file, line: d.line, rule, message }); };
  const theme = isTheme(file);
  const palette = PALETTE_FILES.has(file);
  for (const d of declarations(text)) {
    const { prop, value } = d;
    const custom = prop.startsWith("--");
    const important = /!important/.test(value);
    const v = value.replace(/!important/, "").trim();

    if (important && !/prefers-reduced-motion/.test(d.context)) add("no-important", d, `${prop}: … !important`);

    // Components are written once for every theme; a theme adjusts them from
    // its own file (platform files may pair a theme with an OS).
    if (file.startsWith("components/") && THEME_SELECTOR.test(d.selector) && !seenThemeRule.has(d.selector)) {
      seenThemeRule.add(d.selector);
      add("no-theme-in-component", d, d.selector.replace(/\s+/g, " ").slice(0, 80));
    }

    // Colors: palettes are defined as tokens in base.css and the themes;
    // everything else refers to them.
    if (COLOR_LITERAL.test(v) && !/^(-webkit-)?mask/.test(prop)) {
      const allowed = (palette || theme) && custom;
      if (!allowed) add("no-raw-color", d, `${prop}: ${v.slice(0, 60)}`);
    }
    if (custom) continue;

    if (prop === "font-size" && /\d(px|pt)\b/.test(v) && !v.includes("var(")) add("font-size-token", d, `font-size: ${v}`);

    if (prop === "z-index" && !v.includes("var(")) {
      const n = Number(v);
      // Below 1000 is stacking local to a component (base.css, "Stacking layers").
      if (Number.isFinite(n) && (n > 999 || n < -1)) add("z-index-token", d, `z-index: ${v} (use a --z-* layer)`);
    }

    // Themes may keep signature geometry; components use the radius tokens.
    // A px offset from a token (nested corners: calc(var(--r) - 4px)) is fine.
    const derived = (x) => x.replace(/\b(calc|min|max)\((?=[^()]*var\()[^()]*(\([^()]*\)[^()]*)*\)/g, "").replace(/var\([^)]*\)/g, "");
    if (!theme && /^(border(-(top|bottom)-(left|right))?-radius)$/.test(prop) && /\d+(\.\d+)?px/.test(derived(v))) {
      add("radius-token", d, `${prop}: ${v}`);
    }

    if (!theme && /^(transition|transition-duration|animation|animation-duration)$/.test(prop)) {
      const loop = prop.startsWith("animation") && /infinite|alternate/.test(v);
      for (const m of v.replace(/var\([^)]*\)/g, "").matchAll(/(?<![\w.-])(\d*\.?\d+)(ms|s)\b/g)) {
        const ms = m[2] === "s" ? Number(m[1]) * 1000 : Number(m[1]);
        if (ms === 0 || loop || /prefers-reduced-motion/.test(d.context)) continue;
        add("motion-token", d, `${prop}: ${v.slice(0, 60)} (use --duration-*)`);
        break;
      }
    }

    if (!theme && SPACING_PROPS.test(prop)) {
      for (const m of derived(v).replace(/calc\([^)]*\)/g, "").matchAll(/-?(\d*\.?\d+)px/g)) {
        if (!onSpaceScale(Number(m[1]))) { add("spacing-scale", d, `${prop}: ${v} (${m[0]} is off the scale)`); break; }
      }
    }
  }
  return out;
}

/** Custom properties that are defined but never read, or read but never defined. */
function lintTokens(files) {
  const defined = new Map();
  const used = new Set();
  const optional = new Set();
  const sources = [...files.values()];
  const tsDir = join(ROOT, "src");
  const tsFiles = (function list(dir) {
    return readdirSync(dir).flatMap((name) => {
      const path = join(dir, name);
      return statSync(path).isDirectory() ? list(path) : /\.(tsx?|mjs)$/.test(name) ? [path] : [];
    });
  })(tsDir);
  for (const path of tsFiles) sources.push(readFileSync(path, "utf8"));
  for (const [file, text] of files) {
    for (const d of declarations(text)) if (d.prop.startsWith("--") && !defined.has(d.prop)) defined.set(d.prop, { file, line: d.line });
  }
  for (const raw of sources) {
    const text = raw.replace(/\/\*[\s\S]*?\*\//g, "");
    for (const m of text.matchAll(/var\(\s*(--[\w-]+)\s*(,)?/g)) { used.add(m[1]); if (m[2]) optional.add(m[1]); }
    // Set or read from scripts: style.setProperty("--x"), { "--x": … }, getPropertyValue("--x").
    for (const m of text.matchAll(/["'`](--[\w-]+)["'`]/g)) { used.add(m[1]); defined.has(m[1]) || defined.set(m[1], { file: "(script)", line: 0 }); }
  }
  const out = [];
  for (const [name, at] of defined) {
    if (!used.has(name) && at.file !== "(script)") out.push({ file: at.file, line: at.line, rule: "unused-token", message: name });
  }
  for (const name of used) {
    if (!defined.has(name) && !optional.has(name) && !name.startsWith("--native-palette-")) {
      out.push({ file: "(any)", line: 0, rule: "undefined-token", message: `${name} is read without a fallback but never defined` });
    }
  }
  return out;
}

const files = new Map(walk(STYLES).map((path) => [rel(path), readFileSync(path, "utf8")]));
const violations = [...files].flatMap(([file, text]) => lintFile(file, text)).concat(lintTokens(files));

const counts = {};
for (const v of violations) {
  counts[v.file] ??= {};
  counts[v.file][v.rule] = (counts[v.file][v.rule] ?? 0) + 1;
}

const args = new Set(process.argv.slice(2));
if (args.has("--report")) {
  for (const v of violations) console.log(`src/styles/${v.file}:${v.line}  ${v.rule}  ${v.message}`);
  console.log(`\n${violations.length} violations`);
  process.exit(0);
}
if (args.has("--update")) {
  const sorted = Object.fromEntries(Object.keys(counts).sort().map((f) => [f, Object.fromEntries(Object.entries(counts[f]).sort())]));
  writeFileSync(BASELINE, JSON.stringify(sorted, null, 2) + "\n");
  console.log(`baseline updated: ${violations.length} known violations`);
  process.exit(0);
}

const baseline = existsSync(BASELINE) ? JSON.parse(readFileSync(BASELINE, "utf8")) : {};
const worse = [];
const better = [];
for (const file of new Set([...Object.keys(counts), ...Object.keys(baseline)])) {
  for (const rule of new Set([...Object.keys(counts[file] ?? {}), ...Object.keys(baseline[file] ?? {})])) {
    const now = counts[file]?.[rule] ?? 0;
    const was = baseline[file]?.[rule] ?? 0;
    if (now > was) worse.push({ file, rule, now, was });
    else if (now < was) better.push({ file, rule, now, was });
  }
}
if (worse.length) {
  console.error("Style rules broken (see docs/DESIGN-SYSTEM.md):\n");
  for (const w of worse) {
    console.error(`  ${w.file}  ${w.rule}: ${w.was} → ${w.now}`);
    for (const v of violations.filter((x) => x.file === w.file && x.rule === w.rule)) {
      console.error(`    src/styles/${v.file}:${v.line}  ${v.message}`);
    }
  }
  console.error("\nFix them, or mark an unavoidable one with /* design-ok: reason */.");
  process.exit(1);
}
const total = Object.values(baseline).reduce((s, r) => s + Object.values(r).reduce((a, b) => a + b, 0), 0);
console.log(`styles ok: ${violations.length} known violations (baseline ${total})`);
if (better.length) console.log("Fewer than the baseline; run with --update to lock it in.");
