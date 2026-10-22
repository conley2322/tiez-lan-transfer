// Browser preview of the real React UI with Tauri mocked out, for e2e tests
// (and manual design review). Serves the app from the repo root; the aliases
// swap Tauri APIs and app state for the fixtures in this folder.
//   node ../node_modules/vite/bin/vite.js --config harness/vite.config.mjs --port 1422
import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import { fileURLToPath } from 'node:url';
import { readFileSync } from 'node:fs';

const here = (file) => fileURLToPath(new URL(file, import.meta.url));

// First-run settings defaults, read from the Rust source so the preview starts
// exactly like a fresh install (see mock-backend.ts).
const databaseRs = readFileSync(here('../../src-tauri/src/database.rs'), 'utf8');
const defaultSettings = Object.fromEntries([
  ...[...databaseRs.matchAll(/INSERT OR IGNORE INTO settings \(key, value\) VALUES \('([^']+)', '([^']*)'\)/g)]
    .map((m) => [m[1], m[2]]),
  // CHANGED_DEFAULTS: ("key", "new installs", "existing installs").
  ...[...databaseRs.matchAll(/\("([a-z0-9_.]+)", "([^"]*)", "[^"]*"\),/g)].map((m) => [m[1], m[2]]),
]);

export default defineConfig({
  root: here('../../'),
  // Own dep cache: sharing node_modules/.vite with `npm run dev` (different
  // config) makes each invalidate the other ("504 Outdated Optimize Dep").
  cacheDir: here('../../node_modules/.vite-e2e-harness'),
  // Crawl the app sources up front. The default scans every *.html under the
  // repo (build artifacts included) and misses deps behind the lazy App
  // import; those get discovered on first load and force a full page reload,
  // which wipes the mock backend's state mid-test.
  optimizeDeps: {
    entries: ['index.html', 'src/**/*.{ts,tsx}', '!src/**/*.test.{ts,tsx}', 'e2e/harness/*.ts'],
    // Never pre-bundle Tauri packages: their internal imports of
    // @tauri-apps/api/{core,event,window} would get the mock aliases baked
    // into the dep bundle as a second, stale copy of the mock (separate state,
    // not refreshed on edits). They are plain ESM, so serving them as-is is fine.
    exclude: ['@tauri-apps/api', '@tauri-apps/plugin-dialog', '@tauri-apps/plugin-opener', '@tauri-apps/plugin-process', '@tauri-apps/plugin-updater'],
  },
  plugins: [
    react(),
    {
      // Before the app's own module runs: the first-run settings (an inline
      // script, so it runs before any module) and the Tauri runtime stub.
      name: 'tiez-harness-tauri-internals',
      transformIndexHtml: () => [
        { tag: 'script', children: `window.__TIEZ_DEFAULT_SETTINGS__ = ${JSON.stringify(defaultSettings)};`, injectTo: 'head-prepend' },
        { tag: 'script', attrs: { type: 'module', src: '/e2e/harness/tauri-internals.ts' }, injectTo: 'head-prepend' },
      ],
    },
  ],
  resolve: {
    alias: [
      { find: /^@tauri-apps\/api\/event$/, replacement: here('./mock-tauri-event.ts') },
      { find: /^@tauri-apps\/api\/window$/, replacement: here('./mock-tauri-window.ts') },
      { find: /^@tauri-apps\/api\/core$/, replacement: here('./mock-tauri-core.ts') },
      { find: /^\.\/shared\/hooks\/useAnnouncements$/, replacement: here('./mock-announcements.ts') },
      { find: /^\.\/features\/app\/hooks\/useAppState$/, replacement: here('./preview-state.ts') },
    ],
  },
  server: {
    host: '127.0.0.1',
    port: 1422,
    strictPort: true,
    watch: { ignored: ['**/src-tauri/**', '**/artifacts/**', '**/.git/**', '**/e2e/.e2e/**'] },
  },
});
