// Contract between the frontend's `invoke(...)` calls and the Rust commands in
// src-tauri. The e2e harness mocks `invoke`, and Rust unit tests never see the
// frontend, so a renamed command or argument only failed at runtime before.
//
// Checks every call whose command name and argument object are literals:
//   - the command is registered in main.rs `generate_handler![...]`;
//   - every required (non-Option) Rust parameter is passed;
//   - no argument is passed that the command does not take (Tauri drops it).
// JS keys are camelCase of the Rust snake_case names (Tauri's default; no
// command here uses `rename_all`).
import { readFileSync, readdirSync, statSync } from "node:fs";
import { join, relative } from "node:path";
import ts from "typescript";
import { describe, expect, test } from "vitest";

const ROOT = join(__dirname, "..", "..");
const SRC = join(ROOT, "src");
const RUST_SRC = join(ROOT, "src-tauri", "src");

/** Calls that forward a literal command name: function name → argument positions. */
const INVOKE_WRAPPERS: Record<string, { command: number; args: number }> = {
  invoke: { command: 0, args: 1 },
  applyToggle: { command: 2, args: 3 }, // GeneralSettingsGroup.tsx
};

/** Parameters Tauri injects itself; the frontend never passes them. */
const INJECTED_TYPES = /^(?:tauri::)?(?:State|AppHandle|Window|WebviewWindow|Webview|ipc::Request|Request)\b/;

const walk = (dir: string, keep: (file: string) => boolean): string[] =>
  readdirSync(dir).flatMap((name) => {
    const full = join(dir, name);
    return statSync(full).isDirectory() ? walk(full, keep) : keep(full) ? [full] : [];
  });

const snakeToCamel = (s: string) => s.replace(/_([a-z0-9])/g, (_, c: string) => c.toUpperCase());

// ---- Rust side -------------------------------------------------------------

type RustCommand = { file: string; required: Set<string>; optional: Set<string> };

function registeredCommands(): Set<string> {
  const main = readFileSync(join(RUST_SRC, "main.rs"), "utf8");
  const block = main.match(/generate_handler!\[([\s\S]*?)\]\)/);
  if (!block) throw new Error("generate_handler![...] not found in main.rs");
  return new Set(
    block[1]
      .split(",")
      .map((s) => s.trim().split("::").pop()!)
      .filter((s) => /^[a-z_0-9]+$/.test(s)),
  );
}

/** Splits a parameter list on top-level commas (generics contain commas). */
function splitParams(list: string): string[] {
  const out: string[] = [];
  let depth = 0;
  let current = "";
  for (const ch of list) {
    if (ch === "<" || ch === "(" || ch === "[") depth++;
    if (ch === ">" || ch === ")" || ch === "]") depth--;
    if (ch === "," && depth === 0) {
      out.push(current);
      current = "";
    } else {
      current += ch;
    }
  }
  if (current.trim()) out.push(current);
  return out.map((p) => p.trim()).filter(Boolean);
}

function rustCommands(): Map<string, RustCommand> {
  const commands = new Map<string, RustCommand>();
  for (const file of walk(RUST_SRC, (f) => f.endsWith(".rs"))) {
    const text = readFileSync(file, "utf8");
    const re = /#\[(?:tauri::)?command\b[^\]]*\]\s*(?:#\[[^\]]*\]\s*)*pub\s+(?:async\s+)?fn\s+([a-z_0-9]+)\s*(?:<[^>]*>)?\s*\(/g;
    for (let m = re.exec(text); m; m = re.exec(text)) {
      // Read the parameter list up to the matching ')'.
      let depth = 1;
      let i = re.lastIndex;
      while (depth > 0 && i < text.length) {
        if (text[i] === "(") depth++;
        if (text[i] === ")") depth--;
        i++;
      }
      const required = new Set<string>();
      const optional = new Set<string>();
      const paramList = text.slice(re.lastIndex, i - 1).replace(/\/\/[^\n]*|\/\*[\s\S]*?\*\//g, "");
      for (const param of splitParams(paramList)) {
        const [rawName, ...typeParts] = param.split(":");
        const name = rawName.replace(/^mut\s+/, "").trim();
        const type = typeParts.join(":").trim();
        if (!name || name === "self" || INJECTED_TYPES.test(type)) continue;
        (/^Option\s*</.test(type) ? optional : required).add(snakeToCamel(name.replace(/^_+/, "")));
      }
      commands.set(m[1], { file: relative(ROOT, file), required, optional });
    }
  }
  return commands;
}

// ---- Frontend side ---------------------------------------------------------

type FrontendCall = {
  command: string;
  where: string;
  /** Literal object keys, or null when the args are not an object literal. */
  keys: string[] | null;
  hasArgs: boolean;
  /** Commands sharing this call's argument object (`cond ? "a" : "b"`). */
  siblings: string[];
};

function frontendCalls(): { calls: FrontendCall[]; dynamic: string[] } {
  const calls: FrontendCall[] = [];
  const dynamic: string[] = [];
  const files = walk(SRC, (f) => /\.(ts|tsx)$/.test(f) && !/\.test\.tsx?$/.test(f));
  for (const file of files) {
    const text = readFileSync(file, "utf8");
    const source = ts.createSourceFile(file, text, ts.ScriptTarget.Latest, true, file.endsWith(".tsx") ? ts.ScriptKind.TSX : ts.ScriptKind.TS);
    const visit = (node: ts.Node) => {
      if (ts.isCallExpression(node) && ts.isIdentifier(node.expression)) {
        const wrapper = INVOKE_WRAPPERS[node.expression.text];
        const commandArg = wrapper && node.arguments[wrapper.command];
        if (wrapper && commandArg) {
          const where = `${relative(ROOT, file)}:${source.getLineAndCharacterOfPosition(node.getStart()).line + 1}`;
          // `cond ? "a" : "b"` names two commands; check both.
          const names = ts.isStringLiteralLike(commandArg)
            ? [commandArg.text]
            : ts.isConditionalExpression(commandArg) &&
                ts.isStringLiteralLike(commandArg.whenTrue) &&
                ts.isStringLiteralLike(commandArg.whenFalse)
              ? [commandArg.whenTrue.text, commandArg.whenFalse.text]
              : null;
          if (!names) {
            // `invoke(command, args)` inside a wrapper: its callers are checked instead.
            if (!(node.expression.text === "invoke" && ts.isIdentifier(commandArg))) dynamic.push(where);
          } else {
            const argsNode = node.arguments[wrapper.args];
            let keys: string[] | null = null;
            if (argsNode && ts.isObjectLiteralExpression(argsNode)) {
              keys = [];
              for (const prop of argsNode.properties) {
                if (ts.isSpreadAssignment(prop)) {
                  keys = null; // spread: keys unknown
                  break;
                }
                if (prop.name && (ts.isIdentifier(prop.name) || ts.isStringLiteralLike(prop.name))) keys.push(prop.name.text);
              }
            }
            for (const command of names) calls.push({ command, where, keys, hasArgs: argsNode !== undefined, siblings: names });
          }
        }
      }
      ts.forEachChild(node, visit);
    };
    visit(source);
  }
  return { calls, dynamic };
}

// ---- Checks ----------------------------------------------------------------

const registered = registeredCommands();
const commands = rustCommands();
const { calls, dynamic } = frontendCalls();

describe("Tauri command contract", () => {
  test("parsers found the code (guards against a silent no-op)", () => {
    expect(registered.size).toBeGreaterThan(100);
    expect(commands.size).toBeGreaterThan(100);
    expect(calls.length).toBeGreaterThan(100);
  });

  test("every registered command is defined with #[tauri::command]", () => {
    const undefinedCommands = [...registered].filter((name) => !commands.has(name));
    expect(undefinedCommands).toEqual([]);
  });

  test("every command the frontend invokes is registered in main.rs", () => {
    const missing = calls
      .filter((c) => !registered.has(c.command))
      .map((c) => `${c.command}  (${c.where})`);
    expect(missing).toEqual([]);
  });

  test("required arguments are passed and no unknown argument is sent", () => {
    const problems: string[] = [];
    for (const call of calls) {
      const cmd = commands.get(call.command);
      if (!cmd) continue; // reported by the registration test
      if (call.keys === null && call.hasArgs) continue; // non-literal args: cannot check
      const keys = new Set(call.keys ?? []);
      for (const name of cmd.required) {
        if (!keys.has(name)) problems.push(`${call.command}: missing "${name}"  (${call.where})`);
      }
      // With `cond ? "a" : "b"` one object feeds both commands; a key is
      // unknown only if neither takes it.
      const accepts = (key: string) =>
        call.siblings.some((name) => commands.get(name)?.required.has(key) || commands.get(name)?.optional.has(key));
      for (const key of keys) {
        if (!accepts(key)) {
          problems.push(`${call.command}: unknown argument "${key}"  (${call.where}; Rust: ${cmd.file})`);
        }
      }
    }
    expect(problems).toEqual([]);
  });

  test("no invoke call with a non-literal command name (add it to INVOKE_WRAPPERS)", () => {
    expect(dynamic).toEqual([]);
  });
});
