// Audit-only: two sample announcements when ?ann=1 so the ticker can be inspected.
import type { Announcement } from '../../src/shared/types/announcement';
const show = new URLSearchParams(location.search).get('ann') === '1';
const samples: Announcement[] = [
  { id: 'a1', type: 'update', title: '新版本', message: 'TieZ 1.5 已发布：新增双栏预览与音效设置。' },
  { id: 'a2', type: 'warning', title: '提示', message: '云同步服务将于今晚维护 30 分钟。' },
];
export function useAnnouncements() {
  return { announcements: show ? samples : [], dismissAnnouncement: () => {} };
}
