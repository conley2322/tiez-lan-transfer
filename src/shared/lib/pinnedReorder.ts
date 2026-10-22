/**
 * New pinned_order values after dragging pinned entries into `orderedIds`.
 * The entries reuse the order numbers they already had, so pinned entries a
 * type filter or search hides keep their place among them. Numbers that
 * aren't distinct (older data) can't be reused that way: count down instead.
 */
export const reorderedPinnedOrders = (
    orderedIds: readonly number[],
    currentOrders: ReadonlyMap<number, number>
): [number, number][] => {
    const slots = orderedIds.map((id) => currentOrders.get(id) ?? 0).sort((a, b) => b - a);
    const distinct = new Set(slots).size === slots.length;
    return orderedIds.map((id, index) => [id, distinct ? slots[index] : orderedIds.length - index]);
};
