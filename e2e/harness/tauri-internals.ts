// Loaded before the app (see vite.config.mjs): makes the page look like a
// Tauri webview, so `isTauriRuntime()` code paths — history fetch, settings
// load, event listeners — run against the mock backend instead of being
// skipped. Unaliased @tauri-apps modules (app, webviewWindow, plugins) call
// `__TAURI_INTERNALS__.invoke` and land in the same mock.
import { invoke } from './mock-tauri-core';

let callbackId = 0;
const callbacks = new Map<number, (payload: unknown) => void>();

(window as any).__TAURI_INTERNALS__ = {
  metadata: { currentWindow: { label: 'main' }, currentWebview: { windowLabel: 'main', label: 'main' } },
  invoke: (cmd: string, args?: Record<string, unknown>) => invoke(cmd, args),
  transformCallback: (cb: (payload: unknown) => void) => {
    const id = ++callbackId;
    callbacks.set(id, cb);
    return id;
  },
  unregisterCallback: (id: number) => callbacks.delete(id),
  convertFileSrc: (path: string, protocol = 'asset') => `${protocol}://localhost/${encodeURIComponent(path)}`,
};
