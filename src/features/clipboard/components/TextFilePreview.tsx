import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

const supported = /\.(md|markdown|js|jsx|ts|tsx|mjs|cjs|py|sh|bash|zsh|json|yaml|yml|toml|txt|css|html|rs|go|c|h|cpp|java|sql)$/i;
type Preview = { text: string; truncated: boolean };

export const isTextPreviewable = (path: string) => supported.test(path);

export default function TextFilePreview({ path, language }: { path?: string; language: string }) {
  const [result, setResult] = useState<{ path: string; preview?: Preview; error?: boolean } | null>(null);
  const en = language === "en";
  useEffect(() => {
    if (!path) return;
    let alive = true;
    setResult(null);
    invoke<Preview>("read_text_file_preview", { path }).then(
      preview => { if (alive) setResult({ path, preview }); },
      () => { if (alive) setResult({ path, error: true }); }
    );
    return () => { alive = false; };
  }, [path]);
  if (!path) return null;
  const current = result?.path === path ? result : null;
  return <section className="detail-file-preview" aria-label={en ? "File preview" : "文件预览"}>
    {!current ? <p role="status">{en ? "Loading…" : "正在加载…"}</p> : current.error
      ? <p role="status">{en ? "Unable to preview. The file may be missing, unreadable, or not UTF-8 text." : "无法预览：文件可能已删除、无法读取或不是 UTF-8 文本。"}</p>
      : <>
        <pre className="detail-text is-code">{current.preview?.text || (en ? "Empty file" : "空文件")}</pre>
        {current.preview?.truncated && <p role="status">{en ? "Showing the first 100 KB." : "仅显示前 100 KB 内容。"}</p>}
      </>}
  </section>;
}
