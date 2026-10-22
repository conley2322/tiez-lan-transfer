// Rich text is rendered from the copied HTML: a <style> in a shadow root plus
// the source's style attributes. Both are inline styles. Tauri adds a nonce to
// style-src unless told not to, and a nonce makes WebKit ignore
// 'unsafe-inline': the detail pane then showed tables with no styling at all
// (the list was fine because tables there are drawn as an image).
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { expect, test } from "vitest";

const conf = JSON.parse(readFileSync(join(__dirname, "..", "..", "src-tauri", "tauri.conf.json"), "utf8"));
const security = conf.app.security;

test("inline styles stay allowed: Tauri must not add a nonce to style-src", () => {
  expect(security.csp).toMatch(/style-src [^;]*'unsafe-inline'/);
  const disabled = security.dangerousDisableAssetCspModification;
  expect(disabled === true || (Array.isArray(disabled) && disabled.includes("style-src"))).toBe(true);
});
