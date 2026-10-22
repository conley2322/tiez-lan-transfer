// Audit-only: lets the browser preview render the file transfer view with sample messages.
export * from '../../node_modules/@tauri-apps/api/core.js';
import { backendCommands } from './mock-backend';

const now = Date.now();
const chat = [
  { id: 1, direction: 'in', msg_type: 'text', content: '手机发来的一段文字：会议纪要已整理好。', timestamp: now - 600000, sender_name: '我的手机' },
  { id: 2, direction: 'out', msg_type: 'text', content: '收到，我在电脑上看一下。', timestamp: now - 540000, sender_name: '电脑' },
  { id: 3, direction: 'in', msg_type: 'file', content: '项目报价单.pdf', file_path: '/tmp/项目报价单.pdf', file_size: 482133, timestamp: now - 300000, sender_name: '我的手机' },
  { id: 4, direction: 'out', msg_type: 'file', content: '设计稿-v3.fig', file_path: '/tmp/设计稿-v3.fig', file_size: 12840000, timestamp: now - 120000, sender_name: '电脑' },
];

const proxyErrorPage = '<!DOCTYPE html><html><head><title>502 Bad Gateway</title></head><body><center><h1>502 Bad Gateway</h1></center><hr><center>nginx/1.24.0</center>' + '<!-- a padding to disable MSIE and Chrome friendly error page -->'.repeat(6) + '</body></html>';

const tagItems = Array.from({ length: 14 }, (_, i) => ({ id: 70000 + i, content_type: 'text', content: `带标签的记录 ${i + 1}：会议纪要与待办事项`, preview: `带标签的记录 ${i + 1}：会议纪要与待办事项`, source_app: ['浏览器', '微信', '备忘录'][i % 3], timestamp: Date.now() - i * 3600000, is_pinned: false, tags: ['工作'], use_count: i % 4 }));
export async function invoke(cmd: string, _args?: unknown): Promise<unknown> {
  // Audit/e2e-only: every call, so tests can assert what the UI asked the backend to do.
  ((globalThis as any).__invokeLog ??= []).push({ cmd, args: _args ?? null });
  if (cmd === 'get_all_tags_info') return { 工作: 14, 设计: 6, 灵感: 9, 待办: 3, 代码片段: 21, 密码: 2, 旅行: 4, 读书笔记: 11, 账单: 5, 重要: 7, 会议: 8, 链接收藏: 13 };
  if (cmd === 'get_tag_colors') return { 工作: '#4f7dff', 设计: '#ff85a1' };
  if (cmd === 'get_tag_items') return tagItems;
  if (backendCommands[cmd]) return backendCommands[cmd]((_args ?? {}) as Record<string, unknown>);
  // Setters only push state into Rust; accepting them keeps the console clean.
  if (cmd.startsWith('set_')) return null;
  if (cmd === 'generate_e2e_key') return 'AAECAwQFBgcICQoLDA0ODxAREhMUFRYXGBkaGxwdHh8';
  if (cmd === 'e2e_key_fingerprint') return (_args as { key?: string })?.key?.length === 43 ? '630D-CD29' : null;
  // Audit-only: the exact string AppError produces for an AI provider failure behind a proxy.
  if (cmd === 'call_ai') throw `内部系统错误: AI API error: https://api.deepseek.com/v1/chat/completions - ${proxyErrorPage}`;
  if (cmd === 'get_chat_history') return chat;
  if (cmd === 'get_available_network_interfaces') return [{ name: 'en0', ip: '192.168.1.23', is_virtual: false, label: 'Wi-Fi' }];
  throw new Error(`audit preview: ${cmd} unavailable`);
}
