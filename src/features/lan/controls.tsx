// The controls every LAN page is built from, so a row looks the same in
// 文件传输 as it does in 设备与用户.
//
// Deliberately not the clipboard settings' components: those are welded to
// `.settings-group` markup and to the theme files. These are the same tokens
// (base.css) with a much smaller surface — a labelled row, a switch, a
// segmented picker, a slider, and the avatar every peer list draws.

import type { CSSProperties, ReactNode } from "react";
import type { PeerDisplay } from "../corner/dropTarget";
import { peerColor, peerInitial } from "../corner/dropTarget";

export function LanCard({
    title,
    tone,
    children,
}: {
    title?: string;
    tone?: "note";
    children: ReactNode;
}) {
    return (
        <section className={tone === "note" ? "lan-card is-note" : "lan-card"}>
            {title ? <h3 className="lan-card-title">{title}</h3> : null}
            {children}
        </section>
    );
}

/** Label + hint on the left, control on the right — the row shape 设置 is made of. */
export function LanRow({
    label,
    hint,
    children,
    stack,
}: {
    label: string;
    hint?: string;
    children: ReactNode;
    /** Control goes under the label instead of beside it (long pickers). */
    stack?: boolean;
}) {
    return (
        <div className={stack ? "lan-row is-stacked" : "lan-row"}>
            <div className="lan-row-label">
                <span className="lan-row-name">{label}</span>
                {hint ? <span className="lan-row-hint">{hint}</span> : null}
            </div>
            <div className="lan-row-control">{children}</div>
        </div>
    );
}

export function LanSwitch({
    checked,
    onChange,
    disabled,
    label,
}: {
    checked: boolean;
    onChange: (value: boolean) => void;
    disabled?: boolean;
    label: string;
}) {
    return (
        <button
            type="button"
            role="switch"
            aria-checked={checked}
            aria-label={label}
            className={checked ? "lan-switch is-on" : "lan-switch"}
            disabled={disabled}
            onClick={() => onChange(!checked)}
        >
            <span className="lan-switch-knob" />
        </button>
    );
}

export function LanButton({
    children,
    onClick,
    variant,
    disabled,
    title,
}: {
    children: ReactNode;
    onClick: () => void;
    variant?: "primary" | "danger";
    disabled?: boolean;
    title?: string;
}) {
    const className =
        variant === "primary"
            ? "lan-button is-primary"
            : variant === "danger"
              ? "lan-button is-danger"
              : "lan-button";
    return (
        <button type="button" className={className} onClick={onClick} disabled={disabled} title={title}>
            {children}
        </button>
    );
}

export type SegmentOption<T extends string> = { value: T; label: string };

/** The language / corner / side pickers: one visible choice, no dropdown to open. */
export function LanSegment<T extends string>({
    value,
    options,
    onChange,
    ariaLabel,
}: {
    value: T;
    options: SegmentOption<T>[];
    onChange: (value: T) => void;
    ariaLabel: string;
}) {
    return (
        <div className="lan-segment" role="radiogroup" aria-label={ariaLabel}>
            {options.map((option) => {
                const checked = option.value === value;
                return (
                    <button
                        key={option.value}
                        type="button"
                        role="radio"
                        aria-checked={checked}
                        className={checked ? "lan-segment-item is-on" : "lan-segment-item"}
                        onClick={() => onChange(option.value)}
                    >
                        {option.label}
                    </button>
                );
            })}
        </div>
    );
}

export function LanSelect<T extends string>({
    value,
    options,
    onChange,
    ariaLabel,
}: {
    value: T;
    options: Array<{ value: T; label: string }>;
    onChange: (value: T) => void;
    ariaLabel: string;
}) {
    return (
        <select
            className="lan-select"
            aria-label={ariaLabel}
            value={value}
            onChange={(event) => onChange(event.target.value as T)}
        >
            {options.map((option) => (
                <option key={option.value} value={option.value}>
                    {option.label}
                </option>
            ))}
        </select>
    );
}

export function LanRange({
    value,
    min,
    max,
    onChange,
    ariaLabel,
}: {
    value: number;
    min: number;
    max: number;
    onChange: (value: number) => void;
    ariaLabel: string;
}) {
    return (
        <input
            className="lan-range"
            type="range"
            aria-label={ariaLabel}
            min={min}
            max={max}
            value={value}
            onChange={(event) => onChange(Number(event.target.value))}
        />
    );
}

export function LanPill({
    children,
    tone,
}: {
    children: ReactNode;
    tone?: "accent" | "warn" | "ok" | "muted";
}) {
    return <span className={`lan-pill${tone ? ` is-${tone}` : ""}`}>{children}</span>;
}

/** The peer avatar every list in this app draws the same way: first character,
 *  on the colour that machine's user picked (or the token colour if none). */
export function LanAvatar({
    peer,
    small,
}: {
    peer: PeerDisplay;
    small?: boolean;
}) {
    const color = peerColor(peer);
    const style = color ? ({ background: color } as CSSProperties) : undefined;
    return (
        <span className={small ? "lan-avatar is-small" : "lan-avatar"} style={style} aria-hidden>
            {peerInitial(peer)}
        </span>
    );
}

/** ● green when a beacon came in the last few seconds, grey otherwise. */
export function LanStatus({ online, labels }: { online: boolean; labels: { on: string; off: string } }) {
    return (
        <span className={online ? "lan-status is-on" : "lan-status"}>
            <i aria-hidden />
            {online ? labels.on : labels.off}
        </span>
    );
}

/** ⭐ — a button, because clicking it is the whole point of the star. */
export function LanStar({
    favorite,
    onToggle,
    label,
}: {
    favorite: boolean;
    onToggle: (value: boolean) => void;
    label: string;
}) {
    return (
        <button
            type="button"
            className={favorite ? "lan-star is-on" : "lan-star"}
            aria-label={label}
            aria-pressed={favorite}
            onClick={() => onToggle(!favorite)}
        >
            ★
        </button>
    );
}

/** The 6 fixed tones, as the hexes Rust is willing to store (`normalize_color`).
 *  They are the same values as `--avatar-tone-*` in base.css; a CSS variable
 *  could not be sent over the pairing handshake, so the palette lives here. */
export const AVATAR_TONES = [
    "#3d6fd6",
    "#2b7a4b",
    "#7b5bd0",
    "#b85a1f",
    "#c74b86",
    "#17807f",
];

export function LanColorRow({
    value,
    onChange,
    label,
}: {
    value: string;
    onChange: (color: string) => void;
    label: string;
}) {
    return (
        <div className="lan-colors" role="radiogroup" aria-label={label}>
            {AVATAR_TONES.map((tone) => {
                const checked = tone.toLowerCase() === value.toLowerCase();
                return (
                    <button
                        key={tone}
                        type="button"
                        role="radio"
                        aria-checked={checked}
                        aria-label={tone}
                        className={checked ? "lan-color is-on" : "lan-color"}
                        style={{ background: tone } as CSSProperties}
                        onClick={() => onChange(tone)}
                    />
                );
            })}
        </div>
    );
}
