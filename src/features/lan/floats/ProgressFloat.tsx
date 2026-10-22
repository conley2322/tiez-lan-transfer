// The progress card. One row per file, in both directions, fed by
// 「peer-transfer-progress」.
//
// Rust's unit of work is a group (one batch aimed at one machine), and that is
// also what 取消 and 重试 address — but an inbound file gets its own group, so
// the rows are kept flat and each one carries the group it belongs to. The
// button therefore reads as 「this file」 on an inbound row and as 「this whole
// batch」 on an outbound one, which is exactly what the backend will do.

import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { X } from "lucide-react";
import * as api from "../api";
import type { TransferEvent } from "../api";
import { formatBytes } from "../format";
import type { LanT } from "../lanCopy";

type Row = {
    key: string;
    group: string;
    direction: string;
    peerName: string;
    fileName: string;
    sent: number;
    total: number;
    status: string;
    error: string;
    at: number;
};

/** The card is 168px tall; a folder of 400 files does not get 400 bars. */
const MAX_ROWS = 4;

const running = (status: string) => status === "preparing" || status === "active";

const percent = (row: Row) => {
    if (row.status === "done") return 100;
    if (row.total <= 0) return 0;
    return Math.min(99, Math.round((row.sent / row.total) * 100));
};

export default function ProgressFloat({ t }: { t: LanT }) {
    const [rows, setRows] = useState<Row[]>([]);
    const holdRef = useRef(3);

    const dismiss = useCallback(() => {
        setRows([]);
        api.hideFloat("progress").catch(() => {});
    }, []);

    // How long a finished card stays is 设置's answer, read once: the card lives
    // seconds, and the setting only matters for the next one anyway.
    useEffect(() => {
        api.getFloatState()
            .then((state) => {
                holdRef.current = state.holdSeconds;
            })
            .catch(() => {});
        let unlisten: (() => void) | undefined;
        let cancelled = false;
        (async () => {
            const stop = await listen<TransferEvent>("peer-transfer-progress", (event) => {
                const came = event.payload;
                setRows((current) => {
                    const key = `${came.group}:${came.uploadId || came.fileName}`;
                    const next: Row = {
                        key,
                        group: came.group,
                        direction: came.direction,
                        peerName: came.peerName,
                        fileName: came.fileName,
                        // A retry re-uses the upload id, so a done row is pinned
                        // to the full size rather than to the last chunk's count.
                        sent: came.status === "done" ? came.totalBytes : came.sentBytes,
                        total: came.totalBytes,
                        status: came.status,
                        error: came.error ?? "",
                        at: came.timestamp,
                    };
                    const index = current.findIndex((row) => row.key === key);
                    if (index < 0) return [...current, next];
                    const copy = [...current];
                    copy[index] = next;
                    return copy;
                });
            });
            if (cancelled) stop();
            else unlisten = stop;
        })();
        return () => {
            cancelled = true;
            unlisten?.();
        };
    }, []);

    // 「收发都显，完成自动消失」 — but only when nothing failed. A red row is the
    // one thing on this card that still needs reading, so it waits for a click.
    useEffect(() => {
        if (rows.length === 0) return;
        if (rows.some((row) => running(row.status))) return;
        if (rows.some((row) => row.status === "failed")) return;
        const timer = window.setTimeout(dismiss, holdRef.current * 1000);
        return () => window.clearTimeout(timer);
    }, [rows, dismiss]);

    const visible = useMemo(() => {
        const order = (row: Row) =>
            row.status === "failed" ? 0 : running(row.status) ? 1 : 2;
        return [...rows].sort((a, b) => order(a) - order(b) || b.at - a.at);
    }, [rows]);

    const only = visible.slice(0, MAX_ROWS);
    const direction = rows[0]?.direction ?? "out";
    const peer =
        rows.length > 0 && rows.every((row) => row.peerName === rows[0].peerName)
            ? rows[0].peerName
            : "";

    return (
        <div className="float-card">
            <div className="float-head">
                <span className="float-title">
                    {peer
                        ? direction === "in"
                            ? t("float_receiving", { name: peer })
                            : t("float_sending", { name: peer })
                        : t("float_transfering")}
                </span>
                <button
                    type="button"
                    className="float-close"
                    aria-label={t("float_close")}
                    onClick={dismiss}
                >
                    <X size={14} />
                </button>
            </div>
            <div className="float-body">
                {only.map((row) => (
                    <div className="float-row" key={row.key}>
                        <div className="float-row-name" title={row.error || row.fileName}>
                            {row.fileName}
                        </div>
                        <div className="float-bar">
                            <i
                                className={
                                    row.status === "failed"
                                        ? "float-bar-fill is-bad"
                                        : row.status === "done"
                                          ? "float-bar-fill is-done"
                                          : "float-bar-fill"
                                }
                                style={{ width: `${percent(row)}%` }}
                            />
                        </div>
                        <div className="float-row-foot">
                            <span className="float-row-size">
                                {row.total > 0
                                    ? `${formatBytes(row.sent)} / ${formatBytes(row.total)}`
                                    : t("float_preparing")}
                            </span>
                            {row.status === "failed" ? (
                                <button
                                    type="button"
                                    className="float-act"
                                    onClick={() => {
                                        api.retryTransfer(row.group)
                                            .then(() =>
                                                setRows((current) =>
                                                    current.map((item) =>
                                                        item.group === row.group &&
                                                        item.status === "failed"
                                                            ? { ...item, status: "preparing" }
                                                            : item,
                                                    ),
                                                ),
                                            )
                                            .catch(() => {});
                                    }}
                                >
                                    {t("retry")}
                                </button>
                            ) : running(row.status) ? (
                                <button
                                    type="button"
                                    className="float-act"
                                    onClick={() => {
                                        api.cancelTransfer(row.group).catch(() => {});
                                    }}
                                >
                                    {t("float_cancel")}
                                </button>
                            ) : (
                                <span className="float-row-state">
                                    {row.status === "done" ? t("float_done") : t("float_failed")}
                                </span>
                            )}
                        </div>
                    </div>
                ))}
                {rows.length > MAX_ROWS ? (
                    <div className="float-more">
                        {t("and_more", { count: rows.length - MAX_ROWS })}
                    </div>
                ) : null}
            </div>
        </div>
    );
}
