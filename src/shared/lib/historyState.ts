import type { ClipboardEntry } from "../types";

/** Reconcile a session ID becoming persistent, even if a refresh arrived first. */
export function reconcileHistoryTags(
  history: ClipboardEntry[],
  oldId: number,
  newId: number,
  tags: string[]
): ClipboardEntry[] {
  const source = history.find((item) => item.id === newId)
    ?? history.find((item) => item.id === oldId);
  if (!source) return history;

  const updated = { ...source, id: newId, tags };
  let inserted = false;
  return history.flatMap((item) => {
    if (item.id !== oldId && item.id !== newId) return [item];
    if (inserted) return [];
    inserted = true;
    return [updated];
  });
}
