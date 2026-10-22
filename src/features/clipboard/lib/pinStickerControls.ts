import { WebviewWindow } from "@tauri-apps/api/webviewWindow";
import { ClipboardEntry } from "../../../shared/types";
import { toTauriLocalImageSrc } from "../../../shared/lib/localImageSrc";

export interface PinStickerOptions {
  width?: number;
  height?: number;
}

export const openPinStickerWindow = async (
  item: ClipboardEntry,
  options?: PinStickerOptions
): Promise<WebviewWindow | null> => {
  const imgSrc = toTauriLocalImageSrc(item.content) || item.content;
  if (!imgSrc) return null;

  const label = `pin-sticker-${Date.now()}-${Math.floor(Math.random() * 1000)}`;

  const query = new URLSearchParams({
    window: "pin-sticker",
    id: String(item.id),
    src: imgSrc,
    initialWidth: String(options?.width || 420),
    initialHeight: String(options?.height || 320),
  });

  const width = Math.min(Math.max(options?.width || 420, 160), 800);
  const height = Math.min(Math.max(options?.height || 320, 120), 800);

  try {
    const stickerWindow = new WebviewWindow(label, {
      url: `index.html?${query.toString()}`,
      title: "TieZ Sticker",
      width,
      height,
      minWidth: 120,
      minHeight: 100,
      transparent: true,
      decorations: false,
      alwaysOnTop: true,
      skipTaskbar: false,
      resizable: true,
      shadow: false,
      focus: true,
    });

    return stickerWindow;
  } catch (err) {
    console.error("Failed to create pin sticker window:", err);
    return null;
  }
};
