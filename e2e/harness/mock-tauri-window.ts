// Audit-only: window handle whose every method resolves to a no-op.
export * from '../../node_modules/@tauri-apps/api/window.js';
const handle: any = new Proxy({}, { get: (_t, prop) => (prop === 'label' ? 'main' : async () => () => {}) });
export const getCurrentWindow = () => handle;
