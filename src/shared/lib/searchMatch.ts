import type { ClipboardEntry } from "../types";
import type { ParsedSearchQuery } from "./searchQuery";

const TEXT_TYPES = new Set(["text", "code", "url", "rich_text"]);

export function entryMatchesSearch(
  item: ClipboardEntry,
  parsed: ParsedSearchQuery,
  options?: { isServerSearchResult?: boolean } | boolean
): boolean {
  const isServerMatch =
    typeof options === "boolean"
      ? options
      : options?.isServerSearchResult ??
        ((item as any)._serverSearchMatched === parsed.raw ||
         (item as any).server_matched === true);

  const { term, tagOnly, noteOnly } = parsed;

  if (tagOnly) {
    if (!term) return false;
    return item.tags?.some((tag) => tag.toLowerCase() === term) ?? false;
  }

  if (noteOnly) {
    const note = item.note?.trim() ?? "";
    if (!note) return false;
    if (!term) return true;
    return note.toLowerCase().includes(term);
  }

  if (!term) {
    return true;
  }

  if (item.tags?.some((tag) => tag.toLowerCase().includes(term))) {
    return true;
  }

  if (item.source_app?.toLowerCase().includes(term)) {
    return true;
  }

  if (item.note?.toLowerCase().includes(term)) {
    return true;
  }

  if (TEXT_TYPES.has(item.content_type)) {
    const preview = item.preview?.toLowerCase() ?? "";
    if (preview.includes(term)) {
      return true;
    }
    if (item.content?.toLowerCase().includes(term)) {
      return true;
    }
    if (isServerMatch && item.content?.includes("... [Truncated for speed]")) {
      return true;
    }
    return false;
  }

  return false;
}
