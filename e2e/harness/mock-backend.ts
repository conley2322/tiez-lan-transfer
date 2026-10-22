// Stateful stand-in for the Rust backend, so the UI's real data flow runs in
// the browser: history is fetched with get_clipboard_history, and delete /
// clear / pin / tag / note change what the next fetch returns.
//
// Settings start from the real first-run defaults, which vite.config.mjs reads
// from src-tauri/src/database.rs, so the preview cannot drift from the app.
// Tests can override them per page: ?set=app.arrow_key_selection:true,app.persistent:true
import { emitBackendEvent } from './mock-tauri-event';

type Entry = {
  id: number;
  content_type: string;
  content: string | null;
  preview?: string;
  html_content?: string;
  source_app: string;
  timestamp: number;
  sort_at?: number;
  is_pinned: boolean;
  pinned_order?: number;
  tags: string[];
  note?: string;
  use_count: number;
};

const params = new URLSearchParams(location.search);
const now = Date.now();

// ---- Settings --------------------------------------------------------------

const injectedDefaults = (window as any).__TIEZ_DEFAULT_SETTINGS__ as Record<string, string> | undefined;
// Fail loudly: without the injected defaults the preview would silently run on
// empty settings, which is exactly the drift this file exists to prevent.
if (!injectedDefaults || Object.keys(injectedDefaults).length < 50) {
  throw new Error('e2e harness: default settings were not injected (see vite.config.mjs)');
}
export const DEFAULT_SETTINGS: Record<string, string> = injectedDefaults;

const settings: Record<string, string> = { ...DEFAULT_SETTINGS };
for (const pair of (params.get('set') ?? '').split(',').filter(Boolean)) {
  const i = pair.indexOf(':');
  if (i > 0) settings[pair.slice(0, i)] = pair.slice(i + 1);
}

// ---- History ---------------------------------------------------------------

const samples: Entry[] = [
  { id: 90001, content_type: 'text', content: '项目周会：周五 15:00，先确认需求范围，再安排下周计划。', source_app: '备忘录', timestamp: now - 120000, preview: '项目周会：周五 15:00，先确认需求范围，再安排下周计划。', is_pinned: true, pinned_order: 1, tags: ['工作'], note: '周会常用说明', use_count: 5 },
  { id: 90002, content_type: 'text', content: '你好，资料已收到。我会在明天下午前给你反馈，谢谢。', source_app: '聊天工具', timestamp: now - 240000, preview: '你好，资料已收到。我会在明天下午前给你反馈，谢谢。', is_pinned: false, tags: [], use_count: 2 },
  { id: 90003, content_type: 'url', content: 'https://example.com/project/overview', source_app: '浏览器', timestamp: now - 360000, preview: 'https://example.com/project/overview', is_pinned: false, tags: [], use_count: 0 },
  { id: 90004, content_type: 'rich_text', content: '设计检查：入口是否清楚？操作后是否有反馈？', html_content: '<p><b>设计检查</b>：入口是否清楚？操作后是否有反馈？</p>', source_app: '文档工具', timestamp: now - 480000, preview: '设计检查：入口是否清楚？操作后是否有反馈？', is_pinned: false, tags: ['设计'], use_count: 0 },
];

// ?stress=N: images (data URLs), rich text tables, long text — for layout and performance review.
const makeImage = (hue: number) => {
  const c = document.createElement('canvas');
  c.width = 320; c.height = 180;
  const g = c.getContext('2d')!;
  g.fillStyle = `hsl(${hue},60%,55%)`; g.fillRect(0, 0, 320, 180);
  g.fillStyle = '#fff'; g.font = '28px sans-serif'; g.fillText(`IMG ${hue}`, 20, 100);
  return c.toDataURL('image/png');
};
const stressItems = (count: number): Entry[] => {
  const imgs = Array.from({ length: 12 }, (_, i) => makeImage(i * 30));
  const longText = '长文本内容 Lorem ipsum dolor sit amet. '.repeat(160);
  const paraHtml = Array.from({ length: 30 }, (_, p) => `<p style="margin:4px 0;color:#333">${Array.from({ length: 10 }, (_, k) => `<span style="color:hsl(${(p * 10 + k) * 7 % 360},70%,35%);background:${k % 3 ? 'transparent' : '#fff3c4'};font-weight:${k % 2 ? 400 : 600}">片段${p}-${k} </span>`).join('')}</p>`).join('');
  const rowCount = Number(params.get('rows') || 12); const colCount = Number(params.get('cols') || 5);
  const rows = Array.from({ length: rowCount }, (_, r) => `<tr>${Array.from({ length: colCount }, (_, c) => `<td style="border:1px solid #ccc;padding:2px;font-family:Calibri;font-size:11pt;color:#1f1f1f;background:${(r + c) % 2 ? '#f3f6fb' : '#ffffff'}">R${r}C${c}</td>`).join('')}</tr>`).join('');
  return Array.from({ length: count }, (_, i) => {
    const base = { id: 100000 + i, source_app: ['浏览器', '微信', 'Excel', 'VS Code'][i % 4], timestamp: now - i * 60000, is_pinned: false, tags: i % 7 === 0 ? ['工作'] : [], use_count: 0 };
    if (i % 10 === 0) return { ...base, content_type: 'image', content: imgs[(i / 10) % 12], preview: '[图片]' };
    if (params.get('rich') === 'para') return { ...base, content_type: 'rich_text', content: `网页段落 ${i}`, html_content: paraHtml, preview: `网页段落 ${i}` };
    if (params.get('rich') === '1' || i % 10 === 1) return { ...base, content_type: 'rich_text', content: `表格 ${i}`, html_content: `<table style="border-collapse:collapse">${rows}</table>`, preview: `表格 ${i}` };
    if (i % 10 === 2) return { ...base, content_type: 'text', content: longText, preview: longText.slice(0, 500) };
    if (i === 5 && params.get('huge') === '1') { const huge = 'x'.repeat(2_000_000); return { ...base, content_type: 'text', content: huge, preview: huge.slice(0, 500) }; }
    return { ...base, content_type: 'text', content: `第 ${i} 条记录：普通文本内容`, preview: `第 ${i} 条记录：普通文本内容` };
  });
};

const stressCount = Number(params.get('stress') || 0);
let history: Entry[] = [
  // ?crash=1: a malformed record to exercise the error boundary.
  ...(params.get('crash') === '1' ? [{ id: 1, content_type: 'image', content: null, source_app: '', timestamp: now, is_pinned: false, tags: [], use_count: 0 }] : []),
  ...(stressCount ? stressItems(stressCount) : samples),
].map((e) => ({ ...e, sort_at: e.sort_at ?? e.timestamp }));

/** Pinned first (by pinned_order), then newest — the order the app shows. */
const ordered = () =>
  [...history].sort((a, b) =>
    Number(b.is_pinned) - Number(a.is_pinned) ||
    (b.pinned_order ?? 0) - (a.pinned_order ?? 0) ||
    (b.sort_at ?? 0) - (a.sort_at ?? 0));

const matches = (e: Entry, term: string) =>
  [e.content ?? '', e.preview ?? '', e.source_app, e.note ?? '', ...e.tags].some((s) => s.toLowerCase().includes(term));

const update = (id: number, patch: Partial<Entry>) => {
  history = history.map((e) => (e.id === id ? { ...e, ...patch } : e));
};

let nextId = 200000;
/** Simulates a new copy arriving from the clipboard monitor. */
(window as any).__pushItem = (item: Partial<Entry>) => {
  const entry: Entry = {
    id: item.id ?? nextId++, content_type: 'text', content: '', source_app: '', timestamp: Date.now(),
    is_pinned: false, tags: [], use_count: 0, ...item,
  };
  entry.sort_at = entry.sort_at ?? entry.timestamp;
  history = [entry, ...history.filter((e) => e.id !== entry.id)];
  emitBackendEvent('clipboard-updated', entry);
};

// ---- Commands --------------------------------------------------------------

type Args = Record<string, any>;

export const backendCommands: Record<string, (args: Args) => unknown> = {
  get_settings: () => ({ ...settings }),
  save_setting: ({ key, value }) => { settings[key] = String(value); return null; },

  get_clipboard_history: ({ limit, offset, contentType }) =>
    ordered().filter((e) => !contentType || e.content_type === contentType).slice(offset, offset + limit),
  search_clipboard_history: ({ searchTerm, limit, tagOnly, noteOnly }) => {
    const term = String(searchTerm).toLowerCase();
    return ordered().filter((e) =>
      tagOnly ? e.tags.some((t) => t.toLowerCase() === term)
        : noteOnly ? (e.note ?? '').toLowerCase().includes(term)
          : matches(e, term)).slice(0, limit);
  },
  get_db_count: () => history.length,

  delete_clipboard_entry: ({ id }) => { history = history.filter((e) => e.id !== id); return null; },
  clear_clipboard_history: () => { history = history.filter((e) => e.is_pinned || e.tags.length > 0); return null; },
  toggle_clipboard_pin: ({ id, isPinned }) => {
    update(id, { is_pinned: isPinned, pinned_order: isPinned ? Date.now() : undefined, sort_at: Date.now() });
    return null;
  },
  update_tags: ({ id, tags }) => { update(id, { tags }); return id; },
  update_entry_note: ({ id, note }) => { update(id, { note: note || undefined }); return null; },
  copy_to_clipboard: () => null,
};
