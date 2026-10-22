import { describe, expect, it } from "vitest";
import {
  resolveDropPaths,
  getFilesFromDataTransfer,
  getDroppedTransferItems,
  isExternalFileDrag,
} from "./dragDropHelpers";

describe("File Transfer dragDropHelpers", () => {
  it("resolves drop paths from primitive arrays and objects", () => {
    expect(resolveDropPaths(["/path/to/file1.png", " /path/to/file2.txt "])).toEqual([
      "/path/to/file1.png",
      "/path/to/file2.txt",
    ]);

    expect(resolveDropPaths({ paths: ["/doc/a.pdf", "", "/doc/b.pdf"] })).toEqual([
      "/doc/a.pdf",
      "/doc/b.pdf",
    ]);

    expect(resolveDropPaths(null)).toEqual([]);
    expect(resolveDropPaths({})).toEqual([]);
  });

  it("detects external file drag transfers", () => {
    const fileDt = {
      types: ["Files"],
    } as unknown as DataTransfer;
    expect(isExternalFileDrag(fileDt)).toBe(true);

    const textDt = {
      types: ["text/plain"],
    } as unknown as DataTransfer;
    expect(isExternalFileDrag(textDt)).toBe(false);

    expect(isExternalFileDrag(null)).toBe(false);
  });

  it("extracts files and pending items safely", () => {
    expect(getFilesFromDataTransfer(null)).toEqual([]);
    expect(getDroppedTransferItems(null)).toEqual([]);
  });
});
