import { describe, expect, it } from "vitest";
import { extractRenderableHtmlFragment, stripOfficePreviewNoise } from "./repairHtmlFragment";

const excelSheet = "<style><!--td{color:black;border:none;} .xl66{font-weight:700;background:#DDEBF7;}--></style>";

describe("extractRenderableHtmlFragment", () => {
  it("keeps the head style sheet Excel styles its cells with", () => {
    const html = `<html><head><meta name=ProgId content=Excel.Sheet>${excelSheet}</head><body><table><!--StartFragment--><tr><td class=xl66>月份</td></tr><!--EndFragment--></table></body></html>`;
    expect(extractRenderableHtmlFragment(html)).toBe(`${excelSheet}<tr><td class=xl66>月份</td></tr>`);
  });

  it("keeps head styles when only a body is cut out", () => {
    const html = `<html><head>${excelSheet}</head><body><p>正文</p></body></html>`;
    expect(extractRenderableHtmlFragment(html)).toBe(`${excelSheet}<p>正文</p>`);
  });

  it("does not repeat a style block that is already in the fragment", () => {
    const html = `<html><body>${excelSheet}<p>正文</p></body></html>`;
    expect(extractRenderableHtmlFragment(html)).toBe(`${excelSheet}<p>正文</p>`);
  });

  it("leaves out style blocks with Office metadata only", () => {
    const metadataOnly = "<style>/* Style Definitions */ table.MsoNormalTable {mso-style-name:普通表格; mso-style-priority:99;}</style>";
    const html = `<html><head>${metadataOnly}</head><body><p>正文</p></body></html>`;
    expect(stripOfficePreviewNoise(html)).toBe("<p>正文</p>");
  });
});
