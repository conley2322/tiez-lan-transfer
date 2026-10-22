import type { CSSProperties, InputHTMLAttributes } from "react";

type SettingSwitchProps = Omit<InputHTMLAttributes<HTMLInputElement>, "type" | "className">;

export const SettingSwitch = (props: SettingSwitchProps) => (
    <label className="switch">
        <input className="cb" type="checkbox" {...props} />
        <div className="toggle"><div className="left" /><div className="right" /></div>
    </label>
);

interface SettingRangeProps extends Omit<InputHTMLAttributes<HTMLInputElement>, "type" | "value" | "min" | "max"> {
    value: number;
    min: number;
    max: number;
    valueLabel: string;
}

export const SettingRange = ({ value, min, max, valueLabel, style, ...inputProps }: SettingRangeProps) => {
    const progress = max > min ? Math.min(100, Math.max(0, ((value - min) / (max - min)) * 100)) : 0;
    return (
        <div className="setting-range-control">
            <input
                type="range"
                min={min}
                max={max}
                value={value}
                {...inputProps}
                style={{ ...style, "--range-progress": `${progress}%` } as CSSProperties}
            />
            <span className="setting-range-value">{valueLabel}</span>
        </div>
    );
};
