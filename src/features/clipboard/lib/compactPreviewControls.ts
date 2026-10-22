type CompactPreviewControls = {
  forceHide: () => void;
  warmup: () => void;
  supported: () => boolean;
  warmupSupported: () => boolean;
};

let controls: CompactPreviewControls | null = null;

/**
 * When the clipboard window appears under the cursor, WebView synthesizes
 * mouseenter. Ignore that until the user actually moves the pointer.
 */
let requirePointerMove = false;
let gateOriginX = 0;
let gateOriginY = 0;
let gateHasOrigin = false;
const POINTER_MOVE_ARM_PX = 3;

export const registerCompactPreviewControls = (next: CompactPreviewControls) => {
  controls = next;
};

export const forceHideCompactPreviewWindow = () => {
  controls?.forceHide();
};

export const warmupCompactPreviewWindow = () => {
  controls?.warmup();
};

export const isCompactPreviewWindowSupported = () =>
  controls?.supported() ?? false;

export const isCompactPreviewWarmupSupported = () =>
  controls?.warmupSupported() ?? false;

/** Call when the main clipboard window is shown (hotkey / tray / etc.). */
export const noteCompactPreviewClipboardShown = () => {
  requirePointerMove = true;
  gateHasOrigin = false;
  controls?.forceHide();
};

export const isCompactPreviewHoverArmed = () => !requirePointerMove;

/** Remember the synthetic enter point while the gate is closed. */
export const noteCompactPreviewHoverPoint = (screenX: number, screenY: number) => {
  if (!requirePointerMove) return;
  gateOriginX = screenX;
  gateOriginY = screenY;
  gateHasOrigin = true;
};

/**
 * Returns true when this move just armed hover (caller should schedule preview).
 */
export const noteCompactPreviewPointerMove = (
  screenX: number,
  screenY: number
): boolean => {
  if (!requirePointerMove) return false;

  if (!gateHasOrigin) {
    gateOriginX = screenX;
    gateOriginY = screenY;
    gateHasOrigin = true;
    return false;
  }

  const dx = Math.abs(screenX - gateOriginX);
  const dy = Math.abs(screenY - gateOriginY);
  if (dx < POINTER_MOVE_ARM_PX && dy < POINTER_MOVE_ARM_PX) {
    return false;
  }

  requirePointerMove = false;
  gateHasOrigin = false;
  return true;
};
