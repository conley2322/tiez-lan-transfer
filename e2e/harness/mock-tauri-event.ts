// Audit-only: event API with no backend. The compact preview window gets one
// sample payload so its layout can be inspected (?window=compact-preview&ptype=text|rich).
export * from '../../node_modules/@tauri-apps/api/event.js';
type Handler = (event: { payload: unknown }) => void;
// Looked up on window at call time, never captured at module load, so every
// copy of this module shares one registry. A second copy appeared when vite
// pre-bundled @tauri-apps packages with this alias baked in (now excluded in
// vite.config.mjs); this keeps such a regression from silently splitting the
// app's listeners from the mock backend's events.
const registry = (): Map<string, Set<Handler>> => ((window as any).__tiezEventHandlers ??= new Map());
const handlersFor = (name: string): Set<Handler> => {
  const map = registry();
  if (!map.has(name)) map.set(name, new Set());
  return map.get(name)!;
};

/** Delivers an event the way the Rust backend would (used by mock-backend). */
export const emitBackendEvent = (name: string, payload: unknown) => {
  for (const cb of [...handlersFor(name)]) cb({ payload });
};

/** How many listeners the app has registered for an event (tests wait on this). */
(window as any).__listenerCount = (name: string) => handlersFor(name).size;

export const listen = async (name: string, cb: Handler) => {
  handlersFor(name).add(cb);
  const q = new URLSearchParams(location.search);
  if (name === 'compact-preview-update' && q.get('window') === 'compact-preview') {
    const h = new URLSearchParams(location.hash.slice(1));
    const rich = q.get('ptype') === 'rich';
    setTimeout(() => cb({ payload: {
      requestId: 1,
      contentType: rich ? 'rich_text' : 'text',
      content: rich ? '季度计划：先确认范围，再排期。' : '项目周会：周五 15:00，先确认需求范围，再安排下周计划。会后整理纪要发到群里。',
      htmlContent: rich ? '<h3>季度计划</h3><p><b>先确认范围</b>，再<span style="color:#c00">排期</span>。</p><ul><li>需求评审</li><li>设计走查</li></ul>' : undefined,
      sourceApp: '备忘录', timestamp: Date.now() - 120000, language: 'zh',
      theme: h.get('vrTheme') || 'macos', colorMode: h.get('vrMode') || 'light',
      maxWidth: 520, maxHeight: 320, mediaMaxWidth: 520, mediaMaxHeight: 320, minWidth: 220,
    } }), 80);
  }
  // ?devices=old: one app that predates encryption, one browser that uses it.
  if (name === 'online-devices-updated' && q.get('devices') === 'old') {
    setTimeout(() => cb({ payload: [
      { id: 'a', name: 'iPhone', last_seen: Date.now(), encrypted: false },
      { id: 'b', name: 'Mobile', last_seen: Date.now(), encrypted: true },
    ] }), 80);
  }
  return () => { handlersFor(name).delete(cb); };
};
export const once = async () => () => {};
export const emit = async () => {};
export const emitTo = async () => {};
