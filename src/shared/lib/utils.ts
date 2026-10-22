import type { Locale } from "../types";

const SENSITIVE_MASK = "...";
const URL_PROTOCOL_RE = /^([a-zA-Z][a-zA-Z0-9+\-.]*:\/\/)/;
const EMAIL_RE = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;

interface MaskOptions {
  prefixVisible: number;
  suffixVisible: number;
  maskEmailDomain: boolean;
}

const DEFAULT_MASK_OPTIONS: MaskOptions = {
  prefixVisible: 3,
  suffixVisible: 3,
  maskEmailDomain: false,
};

const MIN_MASKED_CHARS = 2;

const maskMiddleChars = (
  value: string,
  prefixVisibleCount: number,
  suffixVisibleCount: number
) => {
  const chars = Array.from(value);
  if (chars.length <= MIN_MASKED_CHARS) {
    return SENSITIVE_MASK;
  }

  const available = chars.length - MIN_MASKED_CHARS;
  const totalRequested = prefixVisibleCount + suffixVisibleCount;
  let prefix: number;
  let suffix: number;

  if (totalRequested <= available) {
    prefix = prefixVisibleCount;
    suffix = suffixVisibleCount;
  } else {
    prefix =
      totalRequested > 0
        ? Math.floor((available * prefixVisibleCount) / totalRequested)
        : 0;
    suffix = Math.min(suffixVisibleCount, available - prefix);
  }

  return `${chars.slice(0, prefix).join("")}${SENSITIVE_MASK}${chars
    .slice(chars.length - suffix)
    .join("")}`;
};

const clampColorChannel = (value: number) => Math.max(0, Math.min(255, value));

const parseHexColor = (value: string): [number, number, number] | null => {
  const hex = value.replace("#", "").trim();
  if (![3, 4, 6, 8].includes(hex.length)) return null;

  if (hex.length === 3 || hex.length === 4) {
    return [
      parseInt(hex[0] + hex[0], 16),
      parseInt(hex[1] + hex[1], 16),
      parseInt(hex[2] + hex[2], 16),
    ];
  }

  return [
    parseInt(hex.slice(0, 2), 16),
    parseInt(hex.slice(2, 4), 16),
    parseInt(hex.slice(4, 6), 16),
  ];
};

const parseRgbColor = (value: string): [number, number, number] | null => {
  const match = value.match(/^rgba?\(\s*([\d.]+)\s*,\s*([\d.]+)\s*,\s*([\d.]+)/i);
  if (!match) return null;

  return [
    clampColorChannel(Number(match[1])),
    clampColorChannel(Number(match[2])),
    clampColorChannel(Number(match[3])),
  ];
};

const hslToRgb = (h: number, s: number, l: number): [number, number, number] => {
  const hue = ((h % 360) + 360) % 360;
  const saturation = Math.max(0, Math.min(100, s)) / 100;
  const lightness = Math.max(0, Math.min(100, l)) / 100;

  if (saturation === 0) {
    const gray = Math.round(lightness * 255);
    return [gray, gray, gray];
  }

  const c = (1 - Math.abs(2 * lightness - 1)) * saturation;
  const x = c * (1 - Math.abs(((hue / 60) % 2) - 1));
  const m = lightness - c / 2;

  let r = 0;
  let g = 0;
  let b = 0;

  if (hue < 60) {
    r = c; g = x; b = 0;
  } else if (hue < 120) {
    r = x; g = c; b = 0;
  } else if (hue < 180) {
    r = 0; g = c; b = x;
  } else if (hue < 240) {
    r = 0; g = x; b = c;
  } else if (hue < 300) {
    r = x; g = 0; b = c;
  } else {
    r = c; g = 0; b = x;
  }

  return [
    Math.round((r + m) * 255),
    Math.round((g + m) * 255),
    Math.round((b + m) * 255),
  ];
};

const parseHslColor = (value: string): [number, number, number] | null => {
  const match = value.match(/^hsla?\(\s*([-\d.]+)(?:deg)?\s*,\s*([\d.]+)%\s*,\s*([\d.]+)%/i);
  if (!match) return null;
  return hslToRgb(Number(match[1]), Number(match[2]), Number(match[3]));
};

const parseColor = (value: string): [number, number, number] | null => {
  const normalized = value.trim();
  if (!normalized) return null;
  if (normalized.startsWith("#")) return parseHexColor(normalized);
  if (/^rgba?\(/i.test(normalized)) return parseRgbColor(normalized);
  if (/^hsla?\(/i.test(normalized)) return parseHslColor(normalized);
  return null;
};

const rgbToHsl = ([r, g, b]: [number, number, number]): [number, number, number] => {
  const red = r / 255;
  const green = g / 255;
  const blue = b / 255;
  const max = Math.max(red, green, blue);
  const min = Math.min(red, green, blue);
  const lightness = (max + min) / 2;
  const delta = max - min;

  if (delta === 0) return [0, 0, lightness * 100];

  const saturation = delta / (1 - Math.abs(2 * lightness - 1));
  let hue = 0;
  if (max === red) hue = 60 * (((green - blue) / delta) % 6);
  else if (max === green) hue = 60 * ((blue - red) / delta + 2);
  else hue = 60 * ((red - green) / delta + 4);

  return [((hue % 360) + 360) % 360, saturation * 100, lightness * 100];
};

const hslCss = (hue: number, saturation: number, lightness: number) =>
  `hsl(${Math.round(hue)}, ${Math.round(saturation)}%, ${Math.round(lightness)}%)`;

const getContrastRatio = (
  foreground: [number, number, number],
  background: [number, number, number]
) => {
  const foregroundLuminance = getRelativeLuminance(foreground);
  const backgroundLuminance = getRelativeLuminance(background);
  const lighter = Math.max(foregroundLuminance, backgroundLuminance);
  const darker = Math.min(foregroundLuminance, backgroundLuminance);
  return (lighter + 0.05) / (darker + 0.05);
};

const getRelativeLuminance = ([r, g, b]: [number, number, number]) => {
  const toLinear = (channel: number) => {
    const normalized = channel / 255;
    return normalized <= 0.03928
      ? normalized / 12.92
      : ((normalized + 0.055) / 1.055) ** 2.4;
  };

  const red = toLinear(r);
  const green = toLinear(g);
  const blue = toLinear(b);
  return 0.2126 * red + 0.7152 * green + 0.0722 * blue;
};

// Generate one stable fallback color per tag. Theme styling may change the
// chip's shape and decoration, but the tag identity must not change with it.
export const getTagColor = (tag: string, _theme: string) => {
  let hash = 0;
  for (let i = 0; i < tag.length; i++) {
    hash = tag.charCodeAt(i) + ((hash << 5) - hash);
  }

  // Use a larger prime and multiple rotations to ensure strings that are similar
  // (like "tag1" and "tag2") produce very different hues.
  const hue = Math.abs((hash * 137.508 + (hash >> 3)) % 360);

  return `hsl(${hue}, 80%, 55%)`;
};

export const getTagTextColor = (backgroundColor: string, _theme?: string) => {
  const rgb = parseColor(backgroundColor);
  if (!rgb) return "hsl(210 18% 28%)";

  const [hue, saturation] = rgbToHsl(rgb);
  const tagSaturation = Math.max(42, Math.min(78, saturation * 0.82));
  const candidates = [24, 32, 76, 84].map((lightness) => hslCss(hue, tagSaturation, lightness));

  // Keep the foreground in the tag's own hue family while choosing the more
  // readable lightness. This avoids the abrupt black/white inversion between
  // neighboring generated tags.
  return candidates
    .map((candidate) => ({ candidate, rgb: parseHslColor(candidate) }))
    .filter((entry): entry is { candidate: string; rgb: [number, number, number] } => !!entry.rgb)
    .sort((a, b) => getContrastRatio(b.rgb, rgb) - getContrastRatio(a.rgb, rgb))[0]
    ?.candidate || hslCss(hue, tagSaturation, 32);
};

export const getConciseTime = (timestamp: number, language: Locale = "zh") => {
  if (!timestamp || isNaN(timestamp)) return "";

  const ms = timestamp < 1e11 ? timestamp * 1000 : timestamp;
  const now = new Date();
  const date = new Date(ms);
  const diff = now.getTime() - date.getTime();
  const seconds = Math.floor(diff / 1000);

  // Less than 1 minute or future timestamp (due to clock skew)
  if (seconds < 60) {
    if (language === "en") return "Just now";
    if (language === "tw") return "剛剛";
    return "刚刚";
  }

  // Within 1 hour
  if (seconds < 3600) {
    const mins = Math.floor(seconds / 60);
    if (language === "en") return `${mins}m ago`;
    if (language === "tw") return `${mins}分鐘前`;
    return `${mins}分钟前`;
  }

  const pad = (n: number) => n.toString().padStart(2, "0");
  const hours = pad(date.getHours());
  const minutes = pad(date.getMinutes());

  // Same calendar day
  const isSameDay =
    now.getFullYear() === date.getFullYear() &&
    now.getMonth() === date.getMonth() &&
    now.getDate() === date.getDate();

  if (isSameDay) {
    return `${hours}:${minutes}`;
  }

  // Yesterday
  const yesterday = new Date(now);
  yesterday.setDate(yesterday.getDate() - 1);
  const isYesterday =
    yesterday.getFullYear() === date.getFullYear() &&
    yesterday.getMonth() === date.getMonth() &&
    yesterday.getDate() === date.getDate();

  if (isYesterday) {
    if (language === "en") return `Yesterday ${hours}:${minutes}`;
    return `昨天 ${hours}:${minutes}`;
  }

  const month = pad(date.getMonth() + 1);
  const day = pad(date.getDate());

  // Same year: MM-DD
  if (now.getFullYear() === date.getFullYear()) {
    return `${month}-${day}`;
  }

  // Different year: YYYY-MM-DD
  return `${date.getFullYear()}-${month}-${day}`;
};

export const formatSensitivePreview = (
  content: string,
  contentType: string,
  options?: Partial<MaskOptions>
) => {
  if (!content) return "";

  const opts = { ...DEFAULT_MASK_OPTIONS, ...options };

  if (contentType === "url") {
    const protocolMatch = content.match(URL_PROTOCOL_RE);
    if (!protocolMatch) {
      return maskMiddleChars(content, opts.prefixVisible, opts.suffixVisible);
    }

    const protocol = protocolMatch[0];
    const rest = content.slice(protocol.length);
    const maskedRest = maskMiddleChars(rest, opts.prefixVisible, opts.suffixVisible);
    return `${protocol}${maskedRest}`;
  }

  if (EMAIL_RE.test(content.trim())) {
    const email = content.trim();
    const atIndex = email.indexOf("@");
    const localPart = email.slice(0, atIndex);
    const domainPart = email.slice(atIndex + 1);

    const maskedLocal = maskMiddleChars(
      localPart,
      opts.prefixVisible,
      opts.suffixVisible
    );
    if (opts.maskEmailDomain) {
      const maskedDomain = maskMiddleChars(
        domainPart,
        opts.prefixVisible,
        opts.suffixVisible
      );
      return `${maskedLocal}@${maskedDomain}`;
    }

    return `${maskedLocal}@${domainPart}`;
  }

  return maskMiddleChars(content, opts.prefixVisible, opts.suffixVisible);
};
