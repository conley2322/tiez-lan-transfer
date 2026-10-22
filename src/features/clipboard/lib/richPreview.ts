// Shared by list rows and the hover preview window, so both decide alike when
// rich text shows as text and when as an image.

const TABULAR_RICH_HTML_RE = /<(table|tr|td|th|thead|tbody|tfoot|colgroup|col)\b/i;
const SPREADSHEET_SOURCE_RE = /\b(excel|et|wpssheet|wps表格|sheet|spreadsheet|calc)\b/i;
const SPREADSHEET_APP_RE = /(?:^|[\\/])(excel|et|wps|wpssheet|soffice)(?:\.exe|\.app)?$/i;

export const isAnimatedGifSrc = (src?: string | null): boolean => {
  const value = (src || "").trim().toLowerCase();
  if (!value) return false;
  return value.startsWith("data:image/gif") || /\.gif(?:$|[?#])/i.test(value);
};

export const richHtmlLooksTabular = (html?: string): boolean => {
  if (!html) return false;
  return TABULAR_RICH_HTML_RE.test(html);
};

export const isSpreadsheetLikeSource = (...candidates: Array<string | undefined>): boolean => {
  return candidates.some((candidate) => {
    const value = (candidate || "").trim();
    if (!value) return false;
    return SPREADSHEET_APP_RE.test(value) || SPREADSHEET_SOURCE_RE.test(value);
  });
};

/**
 * Whether the source app's bitmap of the content should stand in for it.
 * Text reads better as text; the bitmap wins only for an animated GIF, a
 * spreadsheet range, or content with no text at all.
 */
export const preferRichImageFallback = (options: {
  imageSrc?: string | null;
  text?: string;
  html?: string;
  sourceApp?: string;
  sourceAppPath?: string;
}): boolean => {
  if (isAnimatedGifSrc(options.imageSrc)) return true;
  if (!(options.text || "").trim()) return true;
  return richHtmlLooksTabular(options.html) && isSpreadsheetLikeSource(options.sourceApp, options.sourceAppPath);
};
