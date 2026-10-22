import { expect, test } from "vitest";
import { visibleSelection } from "./selection";

test("entries selected under another tag are left out", () => {
    const shown = [{ id: 3 }, { id: 4 }];

    expect(visibleSelection(shown, new Set([1, 2, 4]))).toEqual([4]);
    expect(visibleSelection(shown, new Set())).toEqual([]);
});
