#!/usr/bin/env node

/**
 * TieZ 本地一键全量测试（发版前跑这个）
 *
 *   npm run preflight              全部
 *   npm run preflight -- --quick   只跑快的（跳过 Rust、原生、文档、生产构建）
 *   npm run preflight -- --no-native --no-docs
 *
 * 每一步都会跑完再汇总，不会在第一个失败处停下；跳过的步骤会写明原因，
 * 不会算作通过。CI（.github/workflows/ci.yml）覆盖其中的 lint / 单测 /
 * 构建 / Rust / UI e2e；原生 smoke 和文档只在这里跑。
 * 自动化覆盖不到的部分见 e2e/MANUAL-CHECKLIST.md。
 */

import { spawnSync } from "node:child_process";
import { existsSync, readFileSync } from "node:fs";
import { resolve } from "node:path";

const rootDir = resolve(import.meta.dirname, "..");
const tauriDir = resolve(rootDir, "src-tauri");
const e2eDir = resolve(rootDir, "e2e");
const docsDir = resolve(rootDir, "../tiez-docs");
const docsE2eDir = resolve(rootDir, "../tiez-e2e");

const args = new Set(process.argv.slice(2));
const quick = args.has("--quick");
const skipNative = quick || args.has("--no-native");
const skipDocs = quick || args.has("--no-docs");

const c = {
  reset: "\x1b[0m", bold: "\x1b[1m", dim: "\x1b[2m",
  green: "\x1b[32m", yellow: "\x1b[33m", red: "\x1b[31m", cyan: "\x1b[36m",
};

/** Runs a command; throws with the tail of its output when it fails. */
function run(cmd, cmdArgs, cwd = rootDir) {
  const res = spawnSync(cmd, cmdArgs, { cwd, encoding: "utf8", shell: process.platform === "win32" });
  if (res.status !== 0) {
    const out = `${res.stdout ?? ""}\n${res.stderr ?? ""}`.trim().split("\n");
    const tail = out.slice(-40).join("\n");
    throw new Error(`\`${cmd} ${cmdArgs.join(" ")}\` 失败 (exit ${res.status ?? res.signal})\n${tail}`);
  }
  return res.stdout ?? "";
}

const otherTiezRunning = () =>
  process.platform === "darwin" && spawnSync("pgrep", ["-x", "tiez-app"]).status === 0;

const steps = [
  {
    name: "发版矩阵与版本配置",
    run: () => run("node", ["scripts/validate-release-matrix.mjs"]),
  },
  {
    name: "ESLint",
    run: () => run("npm", ["run", "lint"]),
  },
  {
    name: "样式规则 (docs/DESIGN-SYSTEM.md)",
    run: () => run("node", ["scripts/lint-styles.mjs"]),
  },
  {
    name: "前端单元测试 (vitest，含 i18n 对齐)",
    run: () => run("npm", ["run", "test"]),
  },
  {
    name: "TypeScript 类型检查",
    run: () => run("npx", ["tsc", "--noEmit"]),
  },
  {
    name: "前端生产构建",
    skip: quick && "--quick",
    run: () => run("npx", ["vite", "build"]),
  },
  {
    name: "Updater 端点 HTTPS",
    run: () => {
      const conf = JSON.parse(readFileSync(resolve(tauriDir, "tauri.conf.json"), "utf8"));
      const endpoints = conf.plugins?.updater?.endpoints ?? [];
      if (endpoints.length === 0) throw new Error("tauri.conf.json 缺少 updater.endpoints");
      const insecure = endpoints.filter((ep) => !ep.startsWith("https://"));
      if (insecure.length) throw new Error(`updater 地址必须是 HTTPS: ${insecure.join(", ")}`);
    },
  },
  {
    name: "Rust 测试 (cargo test)",
    skip: quick && "--quick",
    run: () => run("cargo", ["test", "--locked"], tauriDir),
  },
  {
    name: "UI e2e (预览 harness)",
    run: () => {
      if (!existsSync(resolve(e2eDir, "node_modules"))) {
        run("npm", ["ci"], e2eDir);
        run("npx", ["e2e-web", "install", "chromium"], e2eDir);
      }
      run("npx", ["e2e", "run", "--target", "clipboard-app"], e2eDir);
    },
  },
  {
    name: "原生 smoke (真实启动 app + 捕获剪贴板)",
    skip:
      (skipNative && "--quick / --no-native") ||
      (process.platform !== "darwin" && "仅 macOS") ||
      (otherTiezRunning() && "TieZ 正在运行，请先退出（测试会写剪贴板，且单实例会接管启动）"),
    run: () => run("cargo", ["test", "--locked", "--test", "native_system_e2e", "--", "--ignored"], tauriDir),
  },
  {
    name: "文档站：构建 + 链接检查 + e2e",
    skip:
      (skipDocs && "--quick / --no-docs") ||
      (!existsSync(docsDir) && `未找到 ${docsDir}`) ||
      (!existsSync(resolve(docsE2eDir, "node_modules")) && `未找到 ${docsE2eDir}/node_modules`),
    run: () => {
      run("npm", ["run", "build"], docsDir);
      run("npm", ["run", "check:links"], docsDir);
      run("npx", ["e2e", "run", "--target", "tiez-docs-static", "tests/docs.e2e.ts"], docsE2eDir);
    },
  },
];

const results = [];
const started = Date.now();

for (const [i, step] of steps.entries()) {
  const label = `${c.cyan}[${i + 1}/${steps.length}]${c.reset} ${c.bold}${step.name}${c.reset}`;
  if (step.skip) {
    console.log(`${label} ${c.yellow}跳过${c.reset} ${c.dim}(${step.skip})${c.reset}`);
    results.push({ step, status: "skip", note: step.skip });
    continue;
  }
  process.stdout.write(`${label} ${c.dim}…${c.reset}`);
  const t0 = Date.now();
  try {
    step.run();
    const secs = ((Date.now() - t0) / 1000).toFixed(1);
    console.log(`\r${label} ${c.green}通过${c.reset} ${c.dim}${secs}s${c.reset}`);
    results.push({ step, status: "pass" });
  } catch (err) {
    console.log(`\r${label} ${c.red}失败${c.reset}`);
    results.push({ step, status: "fail", error: err.message });
  }
}

const failed = results.filter((r) => r.status === "fail");
const skipped = results.filter((r) => r.status === "skip");
const passed = results.filter((r) => r.status === "pass");
const total = ((Date.now() - started) / 1000).toFixed(1);

for (const r of failed) {
  console.error(`\n${c.red}${c.bold}✖ ${r.step.name}${c.reset}\n${r.error}`);
}

console.log(
  `\n${c.bold}通过 ${passed.length} · 失败 ${failed.length} · 跳过 ${skipped.length}${c.reset} ${c.dim}(${total}s)${c.reset}`,
);
if (skipped.length) {
  console.log(`${c.yellow}跳过的步骤没有被验证：${skipped.map((r) => r.step.name).join("、")}${c.reset}`);
}
if (!failed.length && !skipped.length) {
  console.log(`${c.green}全部自动化检查通过。发版前再过一遍 e2e/MANUAL-CHECKLIST.md。${c.reset}`);
}
process.exit(failed.length ? 1 : 0);
