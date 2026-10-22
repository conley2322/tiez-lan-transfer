import { test } from "vitest";
import assert from "node:assert/strict";
import { isKeyboardSelectedItem, isItemInteractionActive } from "./itemInteraction.ts";

test("the default first-item target is not visibly selected", () => {
  assert.equal(isKeyboardSelectedItem(1, 1, false), false);
});

test("a remembered mouse-click target does not retain selection", () => {
  assert.equal(isItemInteractionActive(isKeyboardSelectedItem(2, 2, false), false, false, false), false);
});

test("keyboard navigation highlights only its selected target", () => {
  assert.equal(isKeyboardSelectedItem(2, 2, true), true);
  assert.equal(isKeyboardSelectedItem(1, 2, true), false);
  assert.equal(isKeyboardSelectedItem(1, null, true), false);
});

test("keyboard selection survives without hover", () => {
  assert.equal(isItemInteractionActive(true, false, false, false), true);
});

test("tag and note editors remain active without hover", () => {
  assert.equal(isItemInteractionActive(false, true, false, false), true);
  assert.equal(isItemInteractionActive(false, false, true, false), true);
});

test("AI options remain usable without hover", () => {
  assert.equal(isItemInteractionActive(false, false, false, true), true);
});

test("closing an editor restores the inactive state", () => {
  assert.equal(isItemInteractionActive(false, false, false, false), false);
});
