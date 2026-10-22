// Numbers and clocks for the LAN pages.
//
// Sizes are formatted the way a transfer recipient reads them (one decimal,
// binary units), and the relative times go through Intl so 「3 分钟前」 and
// 「3 minutes ago」 come from the platform rather than from a hand-written table.

import type { LanLang } from "./lanCopy";

const UNITS = ["B", "KB", "MB", "GB", "TB"];

export function formatBytes(bytes: number): string {
    const size = Number.isFinite(bytes) && bytes > 0 ? bytes : 0;
    const index = Math.min(UNITS.length - 1, Math.floor(Math.log(size) / Math.log(1024)));
    const value = size / 1024 ** index;
    const text = index === 0 ? String(size) : value >= 100 ? value.toFixed(0) : value.toFixed(1);
    return `${text} ${UNITS[index]}`;
}

const LOCALES: Record<LanLang, string> = {
    zh: "zh-Hans",
    tw: "zh-Hant",
    en: "en",
};

export function relativeTime(millis: number, lang: LanLang): string {
    const formatter = new Intl.RelativeTimeFormat(LOCALES[lang], { numeric: "auto" });
    const seconds = Math.round((millis - Date.now()) / 1000);
    const magnitude = Math.abs(seconds);
    if (magnitude < 60) return formatter.format(seconds, "second");
    const minutes = Math.round(seconds / 60);
    if (Math.abs(minutes) < 60) return formatter.format(minutes, "minute");
    const hours = Math.round(minutes / 60);
    if (Math.abs(hours) < 24) return formatter.format(hours, "hour");
    return formatter.format(Math.round(hours / 24), "day");
}

/** A timestamp for the log table. Month/day/time only: every row is recent by
 *  definition, and the year would eat a column. */
export function formatStamp(millis: number, lang: LanLang): string {
    const date = new Date(millis);
    if (Number.isNaN(date.getTime())) return "—";
    const locale = LOCALES[lang];
    const day = new Intl.DateTimeFormat(locale, { month: "2-digit", day: "2-digit" }).format(date);
    const clock = new Intl.DateTimeFormat(locale, {
        hour: "2-digit",
        minute: "2-digit",
        hour12: false,
    }).format(date);
    return `${day} ${clock}`;
}

/** 「Windows · 192.168.1.8」 — the platform word is already what Rust stored. */
export function platformBadge(platform: string): string {
    const text = platform.trim().toLowerCase();
    if (text.includes("mac")) return "Mac";
    if (text.includes("win")) return "Windows";
    if (text.includes("linux")) return "Linux";
    return platform || "—";
}
