// Preview-only view overrides on top of the real app state. History and
// settings come from the mock backend (mock-backend.ts) through the app's own
// data flow; this file only forces views the preview cannot reach otherwise.
import { useAppState as useRealAppState } from '../../src/features/app/hooks/useAppState';

const params = new URLSearchParams(location.search);
const view = params.get('view');
const themeParam = params.get('theme');
const modeParam = params.get('mode');
const compactParam = params.get('compact') === '1';
const tagFilterParam = params.get('tagfilter') === '1';

export const useAppState = () => {
  const state = useRealAppState();
  const base = {
    ...state,
    ...(themeParam ? { theme: themeParam } : {}),
    ...(modeParam ? { colorMode: modeParam } : {}),
    ...(compactParam ? { compactMode: true, compactShowTime: true } : {}),
    ...(tagFilterParam ? { showTagFilter: true } : {}),
    ...(params.get('snap') === '0' ? { richTextSnapshotPreview: false } : {}),
    ...(params.get('pin') === '1' ? { searchBoxPinned: true } : {}),
    ...(params.get('pin') === '0' ? { searchBoxPinned: false } : {}),
  };
  if (view === 'emoji') return { ...base, showEmojiPanel: true, emojiPanelEnabled: true };
  if (view === 'transfer') {
    return { ...base, chatMode: true, showSettings: true, fileServerEnabled: true, localIp: '192.168.1.23', actualPort: '12345', fileTransferAccessToken: 'audit-token', fileTransferLanKey: 'AAECAwQFBgcICQoLDA0ODxAREhMUFRYXGBkaGxwdHh8' };
  }
  return base;
};
