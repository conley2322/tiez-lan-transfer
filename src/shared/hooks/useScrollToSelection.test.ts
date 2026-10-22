import { afterEach, beforeEach, expect, test, vi } from "vitest";
import { useScrollToSelection } from "./useScrollToSelection";
import type { ClipboardEntry } from "../types";

// Exercise the selection effect with DOM and virtual-scroller test doubles.
vi.mock("react", () => ({ useEffect: (effect: () => void) => effect() }));

const history: ClipboardEntry[] = Array.from({ length: 6 }, (_, index) => ({
  id: index + 10,
  content: "record",
  content_type: "text",
  source_app: "Test",
  timestamp: index,
  preview: "record",
  is_pinned: index < 4,
}));

const scrollIntoView = vi.fn();
const getElementById = vi.fn();
const scroller = { scrollToItem: vi.fn(), scrollToTop: vi.fn(), resetAfterIndex: vi.fn() };
const options = {
  filteredHistory: history,
  selectedIndex: 0,
  isKeyboardMode: true,
  pinnedCount: 4,
  virtualListRef: { current: scroller },
};

beforeEach(() => {
  vi.clearAllMocks();
  getElementById.mockReturnValue({ scrollIntoView });
  vi.stubGlobal("document", { getElementById });
});

afterEach(() => vi.unstubAllGlobals());

test.each([0, 1, 3])("keyboard selection reveals pinned record %i instead of jumping to the top", (selectedIndex) => {
  useScrollToSelection({ ...options, selectedIndex });
  expect(getElementById).toHaveBeenCalledWith(`clipboard-item-${history[selectedIndex].id}`);
  expect(scrollIntoView).toHaveBeenCalledWith({ behavior: "auto", block: "nearest" });
  expect(scroller.scrollToTop).not.toHaveBeenCalled();
  expect(scroller.scrollToItem).not.toHaveBeenCalled();
});

test("a missing pinned element falls back to the top of the virtual list", () => {
  getElementById.mockReturnValue(null);
  useScrollToSelection(options);
  expect(scroller.scrollToTop).toHaveBeenCalledOnce();
  expect(scroller.scrollToItem).not.toHaveBeenCalled();
});

test.each([4, 5])("normal record %i uses its index after the pinned section", (selectedIndex) => {
  useScrollToSelection({ ...options, selectedIndex });
  expect(scroller.scrollToItem).toHaveBeenCalledWith(selectedIndex - options.pinnedCount);
  expect(getElementById).not.toHaveBeenCalled();
  expect(scroller.scrollToTop).not.toHaveBeenCalled();
});

test("lists without a virtual scroller use the supplied element prefix", () => {
  useScrollToSelection({ ...options, selectedIndex: 5, virtualListRef: undefined, idPrefix: "compact-" });
  expect(getElementById).toHaveBeenCalledWith("compact-15");
  expect(scrollIntoView).toHaveBeenCalledWith({ behavior: "auto", block: "nearest" });
});

test.each([
  { isKeyboardMode: false, selectedIndex: 0 },
  { isKeyboardMode: true, selectedIndex: -1 },
  { isKeyboardMode: true, selectedIndex: history.length },
])("mouse selection and invalid indices do not scroll: %j", (selection) => {
  useScrollToSelection({ ...options, ...selection });
  expect(getElementById).not.toHaveBeenCalled();
  expect(scroller.scrollToTop).not.toHaveBeenCalled();
  expect(scroller.scrollToItem).not.toHaveBeenCalled();
});
