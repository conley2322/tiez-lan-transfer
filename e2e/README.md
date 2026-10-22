# UI e2e

在浏览器里跑真实的 React UI，Tauri 后端被 mock 掉（`harness/`）。测试断言 UI
发给后端的调用（mock 把每次 `invoke` 记到 `window.__invokeLog`），不需要启动 app。

```bash
npm ci && npx e2e-web install chromium   # 首次
npm test                                  # 运行器自动启动 harness
npm run test:headed                       # 看着浏览器跑
npm run harness -- --port 1422            # 只开预览，手动看界面
```

- 测试：`tests/app.e2e.ts`；失败时的截图和步骤记录在 `.e2e/results/*/trace.md`
- CI：`.github/workflows/ci.yml` 的 `ui-e2e` job，失败时下载 `ui-e2e` artifact 看 trace
- 全套本地检查：仓库根目录 `npm run preflight`
- 自动化覆盖不到的：`MANUAL-CHECKLIST.md`

## harness 怎么工作

- `harness/tauri-internals.ts`：在 app 启动前注入 `window.__TAURI_INTERNALS__`，让 app 走真实的
  Tauri 代码路径（加载设置、拉取历史、监听事件），所有调用落到下面的 mock 后端。
- `harness/mock-backend.ts`：有状态的 mock 后端。历史记录从 4 条样例开始，删除、清空、固定、
  标签、备注都会改变之后 `get_clipboard_history` 的结果。
- **默认设置 = 真实首次安装的默认值**：`vite.config.mjs` 直接从 `src-tauri/src/database.rs`
  读取 `INSERT OR IGNORE INTO settings` 的默认值注入页面，不会和 app 脱节。
  单个测试需要别的设置时用 URL 参数：`/?set=app.arrow_key_selection:true,ai_enabled:true`。
- 新复制的内容：`window.__pushItem({...})` 会走真实的 `clipboard-updated` 事件；推之前用
  `window.__listenerCount('clipboard-updated') > 0` 确认 app 已在监听。
- `harness/preview-state.ts` 只负责 `?view=emoji|transfer`、`?compact=1` 等预览专用的视图切换。
- mock 没实现的命令会抛 `audit preview: <cmd> unavailable`（`set_*` 一律接受）；测试会忽略这类
  harness 缺口，其他页面错误都会让测试失败。

注意：`vite.config.mjs` 里 `@tauri-apps/*` 必须排除在依赖预构建之外，否则 mock 会被打包进依赖
chunk 形成第二份、不会更新的拷贝，app 的事件监听和 mock 后端的事件就对不上了。
