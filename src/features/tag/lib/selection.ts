/** Ids from `selected` that are in the list shown now; bulk actions act on these only. */
export const visibleSelection = (items: readonly { id: number }[], selected: ReadonlySet<number>): number[] =>
    items.filter((item) => selected.has(item.id)).map((item) => item.id);
