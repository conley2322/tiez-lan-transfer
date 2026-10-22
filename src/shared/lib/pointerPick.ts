/**
 * The row the mouse made the selection, if the current selection came from
 * the pointer. That row is already on screen under the cursor: scrolling to it
 * would slide other rows under the cursor, and the arrow keys haven't taken
 * over. Keyboard selection clears it.
 */
let pointerPickedId: number | null = null;

export const notePointerPick = (id: number | null) => {
  pointerPickedId = id;
};

export const isPointerPick = (id: number | null | undefined) =>
  id != null && id === pointerPickedId;
