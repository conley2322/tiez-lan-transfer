export interface TextStats {
  characters: number;
  words: number;
  lines: number;
}

type WordSegmenter = { segment: (input: string) => Iterable<{ isWordLike?: boolean }> };

let segmenter: WordSegmenter | null | undefined;
const getSegmenter = (): WordSegmenter | null => {
  if (segmenter === undefined) {
    const Segmenter = (Intl as unknown as { Segmenter?: new (locale: string, options: { granularity: "word" }) => WordSegmenter }).Segmenter;
    segmenter = Segmenter ? new Segmenter("zh", { granularity: "word" }) : null;
  }
  return segmenter;
};

/**
 * Character, word and line counts for the detail pane. Characters are code
 * points (an emoji counts once). Words use Intl.Segmenter so Chinese text is
 * split into words instead of counting a whole sentence as one.
 */
export const getTextStats = (text: string): TextStats => {
  if (!text) return { characters: 0, words: 0, lines: 0 };
  const characters = Array.from(text).length;
  const lines = text.split(/\r\n|\r|\n/).length;
  const seg = getSegmenter();
  let words = 0;
  if (seg) {
    for (const part of seg.segment(text)) {
      if (part.isWordLike) words++;
    }
  } else {
    words = text.split(/\s+/).filter(Boolean).length;
  }
  return { characters, words, lines };
};
