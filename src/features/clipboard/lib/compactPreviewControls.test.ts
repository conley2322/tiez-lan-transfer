import { describe, expect, it } from "vitest";
import {
  noteCompactPreviewClipboardShown,
  noteCompactPreviewHoverPoint,
  noteCompactPreviewPointerMove,
  isCompactPreviewHoverArmed,
  registerCompactPreviewControls,
} from "./compactPreviewControls";

describe("Compact Preview Window Controls and Synthetic Pointer Gate", () => {
  it("ignores synthetic pointer enters until user physically moves the cursor", () => {
    noteCompactPreviewClipboardShown();
    expect(isCompactPreviewHoverArmed()).toBe(false);

    // Initial enter point
    noteCompactPreviewHoverPoint(100, 100);
    expect(isCompactPreviewHoverArmed()).toBe(false);

    // Micro jitter less than 3px threshold
    const armedSmall = noteCompactPreviewPointerMove(101, 101);
    expect(armedSmall).toBe(false);
    expect(isCompactPreviewHoverArmed()).toBe(false);

    // Distinct physical movement >= 3px
    const armedReal = noteCompactPreviewPointerMove(105, 105);
    expect(armedReal).toBe(true);
    expect(isCompactPreviewHoverArmed()).toBe(true);
  });

  it("handles controls registration safely", () => {
    let hideCalled = false;
    registerCompactPreviewControls({
      forceHide: () => {
        hideCalled = true;
      },
      warmup: () => {},
      supported: () => true,
      warmupSupported: () => true,
    });

    noteCompactPreviewClipboardShown();
    expect(hideCalled).toBe(true);
  });
});
