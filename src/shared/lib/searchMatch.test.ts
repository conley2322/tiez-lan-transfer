import { describe, expect, it } from "vitest";
import { parseSearchQuery } from "./searchQuery";
import { entryMatchesSearch } from "./searchMatch";
import type { ClipboardEntry } from "../types";

function createMockEntry(overrides: Partial<ClipboardEntry> = {}): ClipboardEntry {
  return {
    id: 1,
    content: "Sample text content",
    content_type: "text",
    created_at: 1700000000,
    is_pinned: false,
    preview: "Sample text preview",
    ...overrides,
  };
}

describe("parseSearchQuery", () => {
  it("parses empty and plain text queries", () => {
    expect(parseSearchQuery("")).toEqual({
      raw: "",
      term: "",
      tagOnly: false,
      noteOnly: false,
    });

    expect(parseSearchQuery("  Hello World  ")).toEqual({
      raw: "Hello World",
      term: "hello world",
      tagOnly: false,
      noteOnly: false,
    });
  });

  it("parses tag: prefix correctly", () => {
    const parsed = parseSearchQuery("tag: work");
    expect(parsed).toEqual({
      raw: "tag: work",
      term: "work",
      tagOnly: true,
      noteOnly: false,
    });
  });

  it("parses note: prefix correctly", () => {
    const parsed = parseSearchQuery("NOTE: Meeting notes");
    expect(parsed).toEqual({
      raw: "NOTE: Meeting notes",
      term: "meeting notes",
      tagOnly: false,
      noteOnly: true,
    });
  });
});

describe("entryMatchesSearch", () => {
  it("matches entry text content and preview", () => {
    const item = createMockEntry({
      preview: "Order ID #12345",
      content: "Complete invoice Order ID #12345 details",
    });

    expect(entryMatchesSearch(item, parseSearchQuery("12345"))).toBe(true);
    expect(entryMatchesSearch(item, parseSearchQuery("nonexistent"))).toBe(false);
  });

  it("matches source application name", () => {
    const item = createMockEntry({
      source_app: "Google Chrome",
    });

    expect(entryMatchesSearch(item, parseSearchQuery("chrome"))).toBe(true);
    expect(entryMatchesSearch(item, parseSearchQuery("safari"))).toBe(false);
  });

  it("matches tag-only queries accurately", () => {
    const item = createMockEntry({
      tags: ["Work", "Urgent"],
    });

    expect(entryMatchesSearch(item, parseSearchQuery("tag:work"))).toBe(true);
    expect(entryMatchesSearch(item, parseSearchQuery("tag:personal"))).toBe(false);
    expect(entryMatchesSearch(item, parseSearchQuery("tag:"))).toBe(false);
  });

  it("matches note-only queries", () => {
    const itemWithNote = createMockEntry({
      note: "Important tax record",
    });
    const itemWithoutNote = createMockEntry({
      note: undefined,
    });

    expect(entryMatchesSearch(itemWithNote, parseSearchQuery("note:tax"))).toBe(true);
    expect(entryMatchesSearch(itemWithNote, parseSearchQuery("note:"))).toBe(true);
    expect(entryMatchesSearch(itemWithoutNote, parseSearchQuery("note:"))).toBe(false);
  });
});
