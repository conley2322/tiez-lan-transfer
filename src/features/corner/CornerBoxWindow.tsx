// The screen-corner drop box (window=corner).
//
// It draws the two states — a small icon and the peer list — and decides which
// machine a file is being dropped on. Rust shows the window only while a file
// is being dragged and hides it again afterwards; the size and screen position
// are its job too (`app/corner_box.rs`), because only it can keep a window
// anchored to a screen corner while it grows.

import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import {
    peerAtPoint,
    peerColor,
    peerInitial,
    peerLabel,
    visiblePeers,
    type CornerPeer,
    type RowRect,
} from "./dropTarget";

type BoxState = {
    enabled: boolean;
    side: string;
    opacity: number;
    favoritesOnly: boolean;
};

/** What the icon says once the drag is over: sent, failed, or just a hint. */
type Outcome = "ok" | "bad" | null;

/** How long the ✓ / ! stays up after a drop before the window hides. */
const RESULT_VISIBLE_MS = 1900;
/** A drop this soon after the drag ends is the one the result belongs to. */
const DROP_GRACE_MS = 2500;

/** 设置 → 角落拖拽框 → 不透明度. A percentage the stylesheet mixes the panel
    background with transparency by, so the text and border stay readable while
    the fill recedes — `opacity` on the window would fade everything. */
const applyOpacity = (percent: number) => {
    const value = Math.min(100, Math.max(0, Math.round(percent)));
    document.documentElement.style.setProperty("--corner-opacity", String(value));
};

export default function CornerBoxWindow() {
    const [peers, setPeers] = useState<CornerPeer[]>([]);
    const [dragging, setDragging] = useState(false);
    const [hoverId, setHoverId] = useState<string | null>(null);
    const [side, setSide] = useState<string>("top-right");
    /** The tooltip / result text the icon carries, not a line next to it. */
    const [note, setNote] = useState<string | null>(null);
    const [outcome, setOutcome] = useState<Outcome>(null);
    const noteTimer = useRef<number | null>(null);
    /** When the last drop that has something to send happened. */
    const sentAt = useRef(0);
    /** Pending "the ✓ has been up long enough" hide. */
    const hideTimer = useRef<number | null>(null);
    const rowsBox = useRef<HTMLDivElement | null>(null);
    const rows = useMemo(() => visiblePeers(peers), [peers]);
    // The drag listener is registered once and lives across peer updates, so
    // what it needs has to be readable without re-registering it.
    const rowCount = useRef(0);
    const rowPeers = useRef<CornerPeer[]>([]);

    useEffect(() => {
        rowCount.current = rows.length;
        rowPeers.current = rows;
    }, [rows]);

    /** Tell Rust to grow into the list (or shrink back) at the current size. */
    const setLayout = useCallback((expanded: boolean) => {
        invoke("set_corner_box_layout", { expanded, rows: rowCount.current }).catch(() => {});
    }, []);

    const closeBox = useCallback(() => {
        setDragging(false);
        setHoverId(null);
        setLayout(false);
    }, [setLayout]);

    const flash = useCallback((text: string, result: Outcome) => {
        setNote(text);
        setOutcome(result);
        if (noteTimer.current !== null) window.clearTimeout(noteTimer.current);
        // The normal path is the hide timer; this only clears a stale tooltip
        // when the window stays up for the Rust-side fallback.
        noteTimer.current = window.setTimeout(() => setNote(null), 2600);
    }, []);

    const cancelHide = useCallback(() => {
        if (hideTimer.current !== null) {
            window.clearTimeout(hideTimer.current);
            hideTimer.current = null;
        }
    }, []);

    const hideBox = useCallback(() => {
        cancelHide();
        sentAt.current = 0;
        setDragging(false);
        setHoverId(null);
        setNote(null);
        setOutcome(null);
        invoke("hide_corner_box").catch(() => {});
    }, [cancelHide]);

    useEffect(() => {
        let unlisteners: Array<() => void> = [];
        let cancelled = false;
        // The filtered list is Rust's (`list_corner_peers` applies
        // 「仅收藏用户」), so a roster change means re-asking, not re-sorting
        // whatever the event happened to carry.
        const poll = () => {
            invoke<CornerPeer[]>("list_corner_peers")
                .then((list) => {
                    if (!cancelled) setPeers(list);
                })
                .catch(() => {});
        };
        poll();
        (async () => {
            const events = await Promise.all([
                listen("peers-changed", poll),
                listen("corner-list-changed", poll),
                listen<string>("corner-side-changed", (event) => setSide(event.payload)),
                listen<number>("corner-opacity-changed", (event) => applyOpacity(event.payload)),
                // Turned off and back on from 设置 reuses this same window, so it
                // has to forget a note left over from before.
                listen("corner-box-enabled-changed", () => {
                    cancelHide();
                    sentAt.current = 0;
                    setDragging(false);
                    setHoverId(null);
                    setNote(null);
                    setOutcome(null);
                    poll();
                }),
                // Rust shows the window when a system-wide file drag starts
                // and hides it when the button comes up. This page never sits
                // on screen between the two.
                listen<boolean>("corner-drag-active", (event) => {
                    if (event.payload) {
                        // A new drag: the previous ✓ or ! is not its result.
                        cancelHide();
                        sentAt.current = 0;
                        setDragging(false);
                        setHoverId(null);
                        setNote(null);
                        setOutcome(null);
                        return;
                    }
                    if (Date.now() - sentAt.current >= DROP_GRACE_MS) {
                        hideBox();
                        return;
                    }
                    // A file just went to a peer: keep the ✓ / ! up for a
                    // moment so the drop does not end in a silent vanishing.
                    setDragging(false);
                    setHoverId(null);
                    setLayout(false);
                    cancelHide();
                    hideTimer.current = window.setTimeout(hideBox, RESULT_VISIBLE_MS);
                }),
            ]);
            if (cancelled) events.forEach((unlisten) => unlisten());
            else unlisteners = events;
        })();
        invoke<BoxState>("get_corner_box_state")
            .then((state) => {
                if (cancelled) return;
                setSide(state.side);
                applyOpacity(state.opacity);
            })
            // A box that cannot ask which corner it is in still works: the
            // icon is centered, and only the arrow would face the wrong way.
            .catch(() => {});
        // A peer only ages into 离线 when a beacon stops arriving, and nothing
        // emits that — the list needs its own clock.
        const timer = window.setInterval(poll, 5000);
        return () => {
            cancelled = true;
            window.clearInterval(timer);
            unlisteners.forEach((unlisten) => unlisten());
        };
    }, [cancelHide, hideBox, setLayout]);

    useEffect(() => {
        const classList = document.documentElement.classList;
        classList.toggle("window-corner-dragging", dragging);
        classList.toggle("window-corner-left", side === "top-left");
        classList.toggle("window-corner-right", side !== "top-left");
    }, [dragging, side]);

    useEffect(
        () => () => {
            if (noteTimer.current !== null) window.clearTimeout(noteTimer.current);
            if (hideTimer.current !== null) window.clearTimeout(hideTimer.current);
        },
        [],
    );

    /** Every avatar row, in the window's own pixel space. */
    const measureRows = (): RowRect[] =>
        Array.from(rowsBox.current?.querySelectorAll<HTMLElement>("[data-peer-id]") ?? []).map(
            (row) => {
                const box = row.getBoundingClientRect();
                return {
                    id: row.dataset.peerId ?? "",
                    left: box.left,
                    top: box.top,
                    right: box.right,
                    bottom: box.bottom,
                };
            },
        );

    const sendTo = useCallback(
        (deviceId: string, paths: string[]) => {
            const target = rowPeers.current.find((peer) => peer.deviceId === deviceId);
            // The other machine's progress box shows this, so it is the name of
            // what was dropped rather than a generic 「一批文件」.
            const batchName =
                paths.length === 1
                    ? (paths[0].split(/[\\/]/).pop() ?? null)
                    : `${paths.length} 个文件`;
            invoke("send_files_to_peers", { peerIds: [deviceId], paths, batchName })
                .then(() => flash(`已发给 · ${target ? peerLabel(target) : "对方"}`, "ok"))
                .catch((error) => flash(`发送失败 ${String(error)}`, "bad"));
        },
        [flash],
    );

    useEffect(() => {
        let unlisten: (() => void) | undefined;
        let cancelled = false;
        (async () => {
            const stop = await getCurrentWebview().onDragDropEvent((event) => {
                const payload = event.payload;
                if (payload.type === "enter") {
                    setDragging(true);
                    setLayout(true);
                    return;
                }
                if (payload.type === "leave") {
                    closeBox();
                    return;
                }
                if (payload.type === "over") {
                    // wry reports the macOS drag point in window-local *points*
                    // even though Tauri calls it a physical position, so
                    // dividing by the scale factor here would land every hover
                    // above the row the file is really over.
                    const { x, y } = payload.position;
                    setHoverId(peerAtPoint(measureRows(), x, y));
                    return;
                }
                if (payload.type === "drop") {
                    const { x, y } = payload.position;
                    const targetId = peerAtPoint(measureRows(), x, y);
                    const { paths } = payload;
                    closeBox();
                    // Off the rows — the hint line, the padding, the icon itself:
                    // nobody was chosen, so nothing is sent.
                    if (targetId && paths.length > 0) {
                        // The native drag monitor reports the button release a
                        // moment later; this is what tells that event to keep
                        // the ✓ up instead of hiding at once.
                        sentAt.current = Date.now();
                        sendTo(targetId, paths);
                    }
                }
            });
            if (cancelled) stop();
            else unlisten = stop;
        })();
        return () => {
            cancelled = true;
            unlisten?.();
        };
    }, [closeBox, sendTo, setLayout]);

    const openSettings = () => invoke("open_settings_window", { tab: "peers" }).catch(() => {});

    if (!dragging) {
        const mark = outcome === "ok" ? "✓" : outcome === "bad" ? "!" : "⇥";
        return (
            <div className="corner-box">
                <div
                    className={[
                        "corner-icon",
                        outcome === "ok" ? "is-ok" : "",
                        outcome === "bad" ? "is-bad" : "",
                    ]
                        .filter(Boolean)
                        .join(" ")}
                    title={note ?? "拖文件到这里发送"}
                >
                    <span className="corner-icon-mark">{mark}</span>
                </div>
            </div>
        );
    }

    if (rows.length === 0) {
        return (
            <div className="corner-box">
                <div className="corner-panel">
                    <div className="corner-tip">还没有配对的电脑</div>
                    <button className="corner-empty" type="button" onClick={openSettings}>
                        <strong>打开设置</strong>
                        <span>设备与用户 · 生成配对码</span>
                    </button>
                </div>
            </div>
        );
    }

    return (
        <div className="corner-box">
            <div className="corner-panel">
                <div className="corner-tip">松手放到头像上 = 发送给 TA</div>
                <div className="corner-rows" ref={rowsBox}>
                    {rows.map((peer) => {
                        const color = peerColor(peer);
                        return (
                            <div
                                key={peer.deviceId}
                                data-peer-id={peer.deviceId}
                                className={[
                                    "corner-row",
                                    hoverId === peer.deviceId ? "is-hot" : "",
                                    peer.online ? "" : "is-offline",
                                ]
                                    .filter(Boolean)
                                    .join(" ")}
                            >
                                <span
                                    className="corner-avatar"
                                    style={color ? { background: color } : undefined}
                                >
                                    {peerInitial(peer)}
                                </span>
                                <span className="corner-name">{peerLabel(peer)}</span>
                                <span className="corner-hint">{peer.online ? "松手发送" : "离线"}</span>
                            </div>
                        );
                    })}
                </div>
            </div>
        </div>
    );
}
