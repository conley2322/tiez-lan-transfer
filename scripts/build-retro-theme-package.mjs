import { execFileSync } from "node:child_process";
import { copyFileSync, mkdtempSync, mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { basename, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const projectRoot = resolve(fileURLToPath(new URL("..", import.meta.url)));
const outputDir = join(projectRoot, "artifacts", "themes");
const outputPath = join(outputDir, "retro.zip");
const workDir = mkdtempSync(join(tmpdir(), "tiez-retro-theme-"));

const metadata = {
  apiVersion: 1,
  id: "retro",
  name: {
    zh: "复古",
    en: "Retro",
    tw: "復古"
  },
  description: {
    zh: "经典机械式立体界面",
    en: "A mechanical retro interface"
  },
  category: "retro",
  version: "1.0.0",
  minAppVersion: "1.0.0",
  supportsCustomBackground: false,
  supportsSurfaceOpacity: false,
  windowMaterial: "none"
};

try {
  const sourceCss = readFileSync(join(projectRoot, "src", "styles", "themes", "retro.css"), "utf8");
  const packageCss = sourceCss.replaceAll("theme-retro", "theme-store-retro");

  writeFileSync(join(workDir, "metadata.json"), `${JSON.stringify(metadata, null, 2)}\n`);
  writeFileSync(join(workDir, "theme.css"), packageCss);
  copyFileSync(join(projectRoot, "docs", "images", "3d.png"), join(workDir, "preview-light.png"));
  copyFileSync(join(projectRoot, "docs", "images", "3d黑.png"), join(workDir, "preview-dark.png"));

  mkdirSync(outputDir, { recursive: true });
  rmSync(outputPath, { force: true });
  const packageFiles = [
    "metadata.json",
    "theme.css",
    "preview-light.png",
    "preview-dark.png"
  ];

  if (process.platform === "win32") {
    const quotedFiles = packageFiles.map((file) => `'${join(workDir, file).replaceAll("'", "''")}'`).join(",");
    const quotedOutput = outputPath.replaceAll("'", "''");
    execFileSync("powershell", [
      "-NoProfile",
      "-Command",
      `Compress-Archive -Path ${quotedFiles} -DestinationPath '${quotedOutput}' -Force`
    ], { stdio: "inherit" });
  } else {
    execFileSync("zip", ["-q", "-j", outputPath, ...packageFiles.map((file) => join(workDir, file))], {
      stdio: "inherit"
    });
  }

  console.log(`Built ${basename(outputPath)} at ${outputPath}`);
} finally {
  rmSync(workDir, { recursive: true, force: true });
}

