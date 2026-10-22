#!/usr/bin/env node
/**
 * Local macOS release build signed with a stable identity from the login
 * keychain, so the Keychain "Always Allow" for TieZ's master key survives
 * rebuilds. An ad-hoc signed build has no stable identity and asks for the
 * password again after every update.
 *
 *   npm run tauri:build:mac-signed            # current architecture
 *   npm run tauri:build:mac-signed -- --target x86_64-apple-darwin
 *
 * Picks "Developer ID Application" when installed (shareable; notarized if
 * APPLE_ID / APPLE_PASSWORD / APPLE_TEAM_ID are set), else "Apple Development"
 * (this Mac only). Set APPLE_SIGNING_IDENTITY to force one.
 */
import { execFileSync, spawnSync } from "node:child_process";

if (process.platform !== "darwin") {
  console.error("macOS only.");
  process.exit(1);
}

function pickIdentity() {
  if (process.env.APPLE_SIGNING_IDENTITY) return process.env.APPLE_SIGNING_IDENTITY;
  const out = execFileSync("security", ["find-identity", "-v", "-p", "codesigning"], { encoding: "utf8" });
  const names = [...out.matchAll(/"([^"]+)"/g)].map((m) => m[1]);
  return (
    names.find((n) => n.startsWith("Developer ID Application:")) ??
    names.find((n) => n.startsWith("Apple Development:")) ??
    null
  );
}

const identity = pickIdentity();
if (!identity) {
  console.error("No code signing identity in the keychain (need Developer ID Application or Apple Development).");
  process.exit(1);
}

const shareable = identity.startsWith("Developer ID Application:");
const notarize = shareable && process.env.APPLE_ID && process.env.APPLE_PASSWORD && process.env.APPLE_TEAM_ID;
console.log(`Signing with: ${identity}`);
console.log(
  shareable
    ? notarize ? "Developer ID + notarization: shareable build." : "Developer ID, not notarized (set APPLE_ID / APPLE_PASSWORD / APPLE_TEAM_ID to notarize)."
    : "Apple Development: for this Mac only, do not distribute.",
);

const args = ["tauri", "build", ...process.argv.slice(2)];
// Updater artifacts need the release signing key, which only CI has. Locally,
// skip them instead of failing after the app is already built.
if (!process.env.TAURI_SIGNING_PRIVATE_KEY) {
  args.push("--config", JSON.stringify({ bundle: { createUpdaterArtifacts: false } }));
}
const res = spawnSync("npx", args, {
  stdio: "inherit",
  env: { ...process.env, APPLE_SIGNING_IDENTITY: identity },
});
process.exit(res.status ?? 1);
