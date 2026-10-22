import { expect, test } from "vitest";
import type { ClipboardEntry } from "../types";
import { quickPasteOrder } from "./quickPasteOrder";

const pin = (id: number, pinned_order: number, sort_at: number) =>
    ({ id, is_pinned: true, pinned_order, sort_at } as ClipboardEntry);

test("numbers follow the pinned order, not the list order", () => {
    // Copying #3 again moved it up the list; Ctrl+1 still pastes #1.
    const list = [pin(3, 1, 900), pin(1, 3, 100), pin(2, 2, 200)];

    expect(quickPasteOrder(list, false, 9).map((item) => item.id)).toEqual([1, 2, 3]);
});

test("a filtered list shows no numbers", () => {
    expect(quickPasteOrder([pin(1, 3, 100)], true, 9)).toEqual([]);
});
