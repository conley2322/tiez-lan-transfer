// Contract between the default a boolean setting falls back to when its key is
// missing from the database: the Rust side (behaviour) and the settings page
// (what the user sees). Seeded keys (database.rs `seed_defaults`) are never
// missing, so only unseeded keys read on both sides can disagree. Example:
// `app.follow_mouse` was unseeded, Rust treated it as on and the page showed off.
import { readFileSync, readdirSync, statSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, test } from "vitest";

const ROOT = join(__dirname, "..", "..");
const RUST_SRC = join(ROOT, "src-tauri", "src");
const POST_INIT = join(ROOT, "src", "shared", "hooks", "useSettingsPostInit.ts");

const walk = (dir: string): string[] =>
  readdirSync(dir).flatMap((name) => {
    const full = join(dir, name);
    return statSync(full).isDirectory() ? walk(full) : full.endsWith(".rs") ? [full] : [];
  });

const databaseRs = readFileSync(join(RUST_SRC, "database.rs"), "utf8");
const seeded = new Set([
  ...[...databaseRs.matchAll(/VALUES \('([^']+)'/g)].map((m) => m[1]),
  // CHANGED_DEFAULTS: ("key", "new installs", "existing installs").
  ...[...databaseRs.matchAll(/\("([a-z0-9_.]+)", "[^"]*", "[^"]*"\),/g)].map((m) => m[1]),
]);

/** Rust: `.get("key")` followed within a few lines by `unwrap_or(Some("true"|"false"))` or `unwrap_or(true|false)`. */
const rustDefaults = new Map<string, boolean>();
for (const file of walk(RUST_SRC)) {
  const text = readFileSync(file, "utf8");
  for (const m of text.matchAll(/\.get(?:_raw)?\((?:conn, )?"([a-z0-9_.]+)"\)(?=([\s\S]{0,200}))/g)) {
    // Lookahead: the tail must not swallow the next `.get(...)`.
    const tail = m[2].split(/;\s*\n|\.get(?:_raw)?\(/)[0];
    const d = tail.match(/unwrap_or\((?:Some\("(true|false)"|(true|false)\))/);
    if (d) rustDefaults.set(m[1], (d[1] ?? d[2]) === "true");
  }
}

/** Frontend: `settings["key"] === "true"` defaults off, `!== "false"` defaults on. */
const frontendDefaults = new Map<string, boolean>();
for (const m of readFileSync(POST_INIT, "utf8").matchAll(/settings\["([^"]+)"\] (===|!==) "(true|false)"/g)) {
  if (m[2] === "===" && m[3] === "true") frontendDefaults.set(m[1], false);
  if (m[2] === "!==" && m[3] === "false") frontendDefaults.set(m[1], true);
}

describe("boolean setting defaults", () => {
  test("parsers found the settings", () => {
    expect(seeded.size).toBeGreaterThan(50);
    expect(rustDefaults.size).toBeGreaterThan(5);
    expect(frontendDefaults.size).toBeGreaterThan(20);
  });

  test("unseeded keys fall back to the same value in Rust and on the settings page", () => {
    const mismatches = [...rustDefaults]
      .filter(([key]) => !seeded.has(key) && frontendDefaults.has(key))
      .filter(([key, rust]) => frontendDefaults.get(key) !== rust)
      .map(([key, rust]) => `${key}: Rust ${rust}, settings page ${frontendDefaults.get(key)}`);
    expect(mismatches).toEqual([]);
  });
});
