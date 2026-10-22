// The slide-to-accept control, borrowed from LocalSend for the reason it is
// borrowed there: a one-click dialog gets confirmed without being read, and an
// incoming file from a stranger is exactly the decision that should cost a
// deliberate motion.
//
// Pointer events only — the cards are opened `focusable(false)`, so nothing here
// can be reached from the keyboard and nothing here may need it.

import { useRef, useState } from "react";
import type { PointerEvent as ReactPointerEvent } from "react";
import { ChevronRight } from "lucide-react";

/** The knob's border-box, matching `.slide-knob` in lan-floats.css (36px + 2px
 *  border on each side). The JS travel is measured against it, so the two have
 *  to agree or the knob stops short of the end of the track. */
const KNOB = 40;
/** How far across the track counts as a decision rather than a slip. */
const COMMIT = 0.62;

export default function SlideToAccept({
    label,
    onAccept,
    disabled,
}: {
    label: string;
    onAccept: () => void;
    disabled?: boolean;
}) {
    const track = useRef<HTMLDivElement>(null);
    const start = useRef(0);
    const [dx, setDx] = useState(0);
    const [dragging, setDragging] = useState(false);

    const limit = () => Math.max(0, (track.current?.clientWidth ?? 0) - KNOB - 4);

    const press = (event: ReactPointerEvent<HTMLButtonElement>) => {
        if (disabled) return;
        start.current = event.clientX;
        setDragging(true);
        event.currentTarget.setPointerCapture(event.pointerId);
    };

    const move = (event: ReactPointerEvent<HTMLButtonElement>) => {
        if (!dragging) return;
        setDx(Math.min(limit(), Math.max(0, event.clientX - start.current)));
    };

    const release = () => {
        setDragging(false);
        if (!disabled && limit() > 0 && dx >= limit() * COMMIT) {
            // Stay at the end: the row is about to leave, and a knob that
            // snapped back would read as the slide having failed.
            setDx(limit());
            onAccept();
            return;
        }
        setDx(0);
    };

    return (
        <div className={dragging ? "slide-track is-dragging" : "slide-track"} ref={track}>
            <span className="slide-label">{label}</span>
            <button
                type="button"
                className={disabled ? "slide-knob is-done" : "slide-knob"}
                style={{ transform: `translateX(${dx}px)` }}
                disabled={disabled}
                aria-label={label}
                onPointerDown={press}
                onPointerMove={move}
                onPointerUp={release}
                onPointerCancel={release}
            >
                <ChevronRight size={18} />
            </button>
        </div>
    );
}
