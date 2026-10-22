import { expect, test } from "vitest";
import { reorderedPinnedOrders } from "./pinnedReorder";

test("pinned entries a filter hides keep their place", () => {
    // Pinned A5 B4 C3 D2 E1; an image filter shows A and B; B is dragged above A.
    const orders = new Map([[1, 5], [2, 4]]);

    expect(reorderedPinnedOrders([2, 1], orders)).toEqual([[2, 5], [1, 4]]);
});

test("equal order numbers from older data count down", () => {
    const orders = new Map([[1, 0], [2, 0], [3, 0]]);

    expect(reorderedPinnedOrders([3, 1, 2], orders)).toEqual([[3, 3], [1, 2], [2, 1]]);
});
