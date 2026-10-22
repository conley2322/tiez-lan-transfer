import { lazy, Suspense, useCallback, useEffect, useRef, useState, type CSSProperties } from "react";
import { Check, Pipette } from "lucide-react";
import { getColorForeground, getCustomPaletteColor } from "../../../shared/config/nativePalettes";
import "./ColorSwatchSelector.css";

const CustomColorPicker = lazy(() => import("./CustomColorPicker"));

interface ColorSwatchSelectorProps {
    label: string;
    value: string;
    options: Array<{ id: string; label: string; color: string }>;
    customLabel: string;
    t: (key: string) => string;
    customColor: string;
    onChange: (value: string) => void;
    onCustomColorChange: (color: string) => void;
}

export default function ColorSwatchSelector({ label, value, options, customLabel,
    t, customColor, onChange, onCustomColorChange }: ColorSwatchSelectorProps) {
    const [pickerOpen, setPickerOpen] = useState(false);
    const closePicker = useCallback(() => setPickerOpen(false), []);
    useEffect(() => { if (!getCustomPaletteColor(value)) setPickerOpen(false); }, [value]);
    const buttons = useRef<Array<HTMLButtonElement | null>>([]);
    const customValue = `custom:${customColor}`;
    const choices = [...options, { id: customValue, label: customLabel, color: customColor }];
    const selected = getCustomPaletteColor(value) ? choices.length - 1 : Math.max(0, choices.findIndex(item => item.id === value));

    return (
        <div className="color-swatch-selector">
            <div className="color-swatch-options" role="radiogroup" aria-label={label}>
                {choices.map((choice, index) => {
                    const isCustom = index === choices.length - 1;
                    const checked = selected === index;
                    return (
                        <button
                            key={isCustom ? "custom" : choice.id}
                            ref={node => { buttons.current[index] = node; }}
                            type="button"
                            role="radio"
                            aria-checked={checked}
                            aria-label={choice.label}
                            aria-haspopup={isCustom ? "dialog" : undefined}
                            aria-expanded={isCustom ? pickerOpen : undefined}
                            tabIndex={checked ? 0 : -1}
                            className={`color-swatch-option${checked ? " is-selected" : ""}`}
                            style={{ "--swatch-color": choice.color, "--swatch-foreground": getColorForeground(choice.color) } as CSSProperties}
                            onClick={() => {
                                onChange(choice.id);
                                if (isCustom) setPickerOpen(open => !open);
                            }}
                            onKeyDown={event => {
                                const delta = ["ArrowRight", "ArrowDown"].includes(event.key) ? 1
                                    : ["ArrowLeft", "ArrowUp"].includes(event.key) ? -1 : 0;
                                if (!delta && event.key !== "Home" && event.key !== "End") return;
                                event.preventDefault();
                                event.stopPropagation();
                                const next = event.key === "Home" ? 0 : event.key === "End" ? choices.length - 1
                                    : (index + delta + choices.length) % choices.length;
                                onChange(choices[next].id);
                                buttons.current[next]?.focus({ preventScroll: true });
                            }}
                        >
                            <span className={`color-swatch-circle${isCustom ? " is-custom" : ""}`} aria-hidden="true">
                                {isCustom ? <Pipette size={13} strokeWidth={1.8} /> : checked ? <Check size={14} strokeWidth={2.5} /> : null}
                            </span>
                            <span className="color-swatch-label">{choice.label}</span>
                        </button>
                    );
                })}
            </div>
            {pickerOpen && buttons.current[choices.length - 1] && (
                <Suspense fallback={null}><CustomColorPicker
                    anchor={buttons.current[choices.length - 1]!}
                    color={customColor}
                    t={t}
                    onChange={onCustomColorChange}
                    onClose={closePicker}
                /></Suspense>
            )}
        </div>
    );
}
