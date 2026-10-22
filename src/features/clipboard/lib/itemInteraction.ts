/** A remembered paste target is not a visible selection until keyboard navigation starts. */
export function isKeyboardSelectedItem(
  itemId: number,
  selectedItemId: number | null,
  isKeyboardMode: boolean
): boolean {
  return isKeyboardMode && selectedItemId !== null && itemId === selectedItemId;
}

/** Editors remain active when the pointer leaves; hover itself is handled by CSS. */
export function isItemInteractionActive(
  isKeyboardSelected: boolean,
  isEditingTags: boolean,
  isEditingNote: boolean,
  isAiOptionsOpen: boolean
): boolean {
  return isKeyboardSelected || isEditingTags || isEditingNote || isAiOptionsOpen;
}
