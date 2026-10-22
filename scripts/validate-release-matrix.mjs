import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { spawnSync } from "node:child_process";

const combinations = [
  ["windows", "stable"],
  ["windows", "beta"],
  ["macos", "stable"],
  ["macos", "beta"],
  // Restore the repository's default generated state.
  ["windows", "stable"],
];

for (const [platform, channel] of combinations) {
  const result = spawnSync(
    process.execPath,
    [resolve("scripts/build-release.mjs"), platform, channel, "--prepare-only"],
    {
      stdio: "inherit",
      env: {
        ...process.env,
        CI: "false",
        GITHUB_ACTIONS: "false",
        // A pushed release tag belongs to one platform and channel; checking
        // it against all four failed every release tag. It is checked once
        // below instead.
        GITHUB_REF_TYPE: "",
        GITHUB_REF_NAME: "",
      },
    },
  );
  if (result.status !== 0) process.exit(result.status ?? 1);
}

const tag = (process.env.GITHUB_REF_NAME || "").trim();
if (tag && process.env.GITHUB_REF_TYPE === "tag") {
  const versions = JSON.parse(readFileSync(resolve("versions.json"), "utf8"));
  const expected = [
    `v${versions.stable.windows}`,
    `mac-v${versions.stable.macos}`,
    `beta-v${versions.beta.windows}`,
    `beta-mac-v${versions.beta.macos}`,
  ];
  if (!expected.includes(tag)) {
    console.error(`[validate-release-matrix] tag ${tag} matches no version in versions.json; expected one of ${expected.join(", ")}`);
    process.exit(1);
  }
  console.log(`[validate-release-matrix] tag ${tag} matches versions.json`);
}

console.log("[validate-release-matrix] stable and beta metadata passed for Windows and macOS");
