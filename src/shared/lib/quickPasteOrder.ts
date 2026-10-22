import type { ClipboardEntry } from "../types";

/**
 * Pinned entries in quick-paste slot order, as the backend picks them for
 * Ctrl+1…9 (paste_history_item_by_index): pinned order, then most recent.
 * A filtered list holds only some pinned entries, so its numbers would be
 * wrong: none then.
 */
export const quickPasteOrder = (
    items: readonly ClipboardEntry[],
    listIsFiltered: boolean,
    slots: number
): ClipboardEntry[] => {
    if (listIsFiltered) return [];
    return items
        .filter((item) => item.is_pinned)
        .sort((a, b) =>
            (b.pinned_order ?? 0) - (a.pinned_order ?? 0)
            || (b.sort_at ?? b.timestamp) - (a.sort_at ?? a.timestamp)
            || b.id - a.id
        )
        .slice(0, slots);
};
