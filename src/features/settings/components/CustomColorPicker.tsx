import { useEffect, useId, useRef, useState } from "react";
import { createPortal } from "react-dom";
import { colorToHsl, hslToColor, normalizeCustomColor } from "../../../shared/config/nativePalettes";

interface CustomColorPickerProps {
    anchor: HTMLButtonElement;
    color: string;
    t: (key: string) => string;
    onChange: (color: string) => void;
    onClose: () => void;
}

export default function CustomColorPicker({ anchor, color, t, onChange, onClose }: CustomColorPickerProps) {
    const panel = useRef<HTMLDivElement>(null);
    const id = useId();
    const [hsl, setHsl] = useState(() => colorToHsl(color));
    const [draft, setDraft] = useState(color);
    const [position, setPosition] = useState({ top: 0, left: 0 });
    const invalid = !normalizeCustomColor(draft.startsWith("#") ? draft : `#${draft}`);

    useEffect(() => {
        // Keep exact slider positions when RGB rounding would shift hue or saturation.
        setHsl(previous => hslToColor(...previous) === color ? previous : colorToHsl(color));
        setDraft(color);
    }, [color]);

    useEffect(() => {
        const place = () => {
            const bounds = anchor.getBoundingClientRect();
            const height = panel.current?.offsetHeight || 235;
            const width = Math.min(236, window.innerWidth - 20);
            const below = bounds.bottom + 8;
            setPosition({
                left: Math.max(10, Math.min(bounds.right - width, window.innerWidth - width - 10)),
                top: Math.max(10, below + height <= window.innerHeight - 10 ? below : bounds.top - height - 8)
            });
        };
        const dismiss = (event: PointerEvent) => {
            if (!panel.current?.contains(event.target as Node) && !anchor.contains(event.target as Node)) onClose();
        };
        place();
        const observer = new ResizeObserver(place);
        if (panel.current) observer.observe(panel.current);
        panel.current?.querySelector<HTMLInputElement>('input[type="range"]')?.focus({ preventScroll: true });
        window.addEventListener("resize", place);
        document.addEventListener("scroll", place, true);
        document.addEventListener("pointerdown", dismiss, true);
        return () => {
            observer.disconnect();
            window.removeEventListener("resize", place);
            document.removeEventListener("scroll", place, true);
            document.removeEventListener("pointerdown", dismiss, true);
        };
    }, [anchor, onClose]);

    const closeAndFocus = () => { onClose(); anchor.focus({ preventScroll: true }); };
    return createPortal(
        <div ref={panel} className="custom-color-picker" role="dialog" aria-modal="false" aria-labelledby={id}
            style={position}
            onKeyDown={event => {
                event.stopPropagation();
                if (event.key === "Escape") { event.preventDefault(); closeAndFocus(); }
            }}>
            <div className="custom-color-picker-heading">
                <span id={id}>{t('native_palette_choose_color')}</span>
                <span className="custom-color-preview" style={{ backgroundColor: color }} aria-hidden="true" />
            </div>
            {['hue', 'saturation', 'lightness'].map((channel, index) => (
                <label className="custom-color-channel" key={channel}>
                    <span>{t(`native_palette_${channel}`)}</span>
                    <input type="range" min={0} max={index === 0 ? 360 : 100} step={1}
                        value={Math.round(hsl[index])}
                        aria-label={t(`native_palette_${channel}`)}
                        style={{ background: index === 0
                            ? 'linear-gradient(to right, #f00, #ff0, #0f0, #0ff, #00f, #f0f, #f00)'
                            : index === 1 ? `linear-gradient(to right, ${hslToColor(hsl[0], 0, hsl[2])}, ${hslToColor(hsl[0], 100, hsl[2])})`
                                : `linear-gradient(to right, #000, ${hslToColor(hsl[0], hsl[1], 50)}, #fff)` }}
                        onChange={event => {
                            const next: [number, number, number] = [...hsl];
                            next[index] = Number(event.target.value);
                            setHsl(next);
                            onChange(hslToColor(...next));
                        }} />
                </label>
            ))}
            <label className="custom-color-value">
                <span>{t('native_palette_hex')}</span>
                <input type="text" value={draft} spellCheck={false} autoComplete="off" maxLength={7}
                    aria-invalid={invalid} aria-describedby={invalid ? `${id}-error` : undefined}
                    onChange={event => {
                        const value = event.target.value;
                        setDraft(value);
                        const parsed = normalizeCustomColor(value.startsWith('#') ? value : `#${value}`);
                        if (parsed) onChange(parsed);
                    }} />
            </label>
            {invalid && <span className="custom-color-error" id={`${id}-error`} role="status">{t('native_palette_hex_hint')}</span>}
            <button type="button" className="custom-color-done" onClick={closeAndFocus}>{t('native_palette_done')}</button>
        </div>, document.body
    );
}
