import { test } from "vitest";
import assert from "node:assert/strict";
import { reconcileHistoryTags } from "./historyState.ts";
import type { ClipboardEntry } from "../types";

const entry = (id: number, tags: string[] = []): ClipboardEntry => ({
  id, tags, content: "same content", content_type: "text", source_app: "Test",
  timestamp: 1, preview: "same content", is_pinned: false,
});

test("promoting a session entry replaces its ID without adding a row", () => {
  const result = reconcileHistoryTags([entry(-1), entry(2)], -1, 10, ["tag"]);
  assert.deepEqual(result.map((item) => item.id), [10, 2]);
  assert.deepEqual(result[0].tags, ["tag"]);
});

test("a refresh arriving before the command reply is safe", () => {
  const result = reconcileHistoryTags([entry(10), entry(2)], -1, 10, ["tag"]);
  assert.deepEqual(result.map((item) => item.id), [10, 2]);
  assert.deepEqual(result[0].tags, ["tag"]);
});

test("old and persistent IDs are merged, retaining server fields", () => {
  const result = reconcileHistoryTags([
    entry(-1), { ...entry(10), use_count: 7 }, entry(2), entry(10),
  ], -1, 10, ["tag"]);
  assert.deepEqual(result.map((item) => item.id), [10, 2]);
  assert.equal(result[0].use_count, 7);
});

test("existing duplicate persistent IDs are collapsed when editing tags", () => {
  assert.equal(reconcileHistoryTags([entry(10), entry(10)], 10, 10, []).length, 1);
});

test("identical content belonging to other IDs is preserved", () => {
  assert.equal(reconcileHistoryTags([entry(10), entry(11)], 10, 10, ["tag"]).length, 2);
});

test("a late reply does not resurrect a deleted or filtered entry", () => {
  const history = [entry(2)];
  assert.equal(reconcileHistoryTags(history, -1, 10, ["tag"]), history);
});
