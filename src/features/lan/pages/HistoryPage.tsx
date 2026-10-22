// 传输记录 — the log Rust writes when a file finishes or fails.
//
// Rows are grouped by the batch they belong to, because that is also the unit
// 「重试」 works on: `retry_peer_transfer` takes a group and re-sends every file
// of it to the same machine. One button per row would offer something the
// backend cannot do.

import { useCallback, useEffect, useMemo, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { Eraser } from "lucide-react";
import ConfirmDialog from "../../../shared/components/ConfirmDialog";
import * as api from "../api";
import type { TransferEvent, TransferRow } from "../api";
import { formatBytes, formatStamp } from "../format";
import { LanButton, LanCard, LanPill } from "../controls";
import type { SettingsCtx } from "../LanSettingsWindow";

type Batch = { group: string; rows: TransferRow[] };

function groupBatches(rows: TransferRow[]): Batch[] {
    const byGroup = new Map<string, TransferRow[]>();
    for (const row of rows) {
        const list = byGroup.get(row.groupId);
        if (list) list.push(row);
        else byGroup.set(row.groupId, [row]);
    }
    return [...byGroup.entries()].map(([group, list]) => ({ group, rows: list }));
}

export default function HistoryPage({ ctx }: { ctx: SettingsCtx }) {
    const { t, lang } = ctx;
    const [rows, setRows] = useState<TransferRow[]>([]);
    const [notice, setNotice] = useState("");
    const [confirming, setConfirming] = useState(false);

    const refresh = useCallback(() => {
        api.listTransfers(200)
            .then(setRows)
            .catch(() => {});
    }, []);

    useEffect(() => {
        refresh();
        let unlisten: (() => void) | undefined;
        let cancelled = false;
        let timer = 0;
        (async () => {
            // A folder is hundreds of events, and only its last row is a new
            // log line, so the reload waits for a lull instead of firing per file.
            const stop = await listen<TransferEvent>("peer-transfer-progress", (event) => {
                if (event.payload.status !== "done" && event.payload.status !== "failed") return;
                window.clearTimeout(timer);
                timer = window.setTimeout(refresh, 500);
            });
            if (cancelled) stop();
            else unlisten = stop;
        })();
        return () => {
            cancelled = true;
            window.clearTimeout(timer);
            unlisten?.();
        };
    }, [refresh]);

    const batches = useMemo(() => groupBatches(rows), [rows]);

    const retry = (group: string) => {
        api.retryTransfer(group)
            .then(() => setNotice(t("retry_started")))
            .catch((error: unknown) =>
                setNotice(t("retry_failed", { reason: String(error) })),
            );
    };

    const clear = () => {
        setConfirming(false);
        api.clearTransfers()
            .then((count) => {
                setNotice(t("cleared", { count }));
                setRows([]);
            })
            .catch(() => {});
    };

    return (
        <>
            <h2 className="lan-page-title">{t("history_title")}</h2>

            <div className="lan-page-actions">
                <LanButton
                    variant="danger"
                    onClick={() => setConfirming(true)}
                    disabled={rows.length === 0}
                >
                    <Eraser size={14} />
                    {t("clear_records")}
                </LanButton>
            </div>

            {notice ? <p className="lan-inline-result">{notice}</p> : null}

            <LanCard>
                {batches.length === 0 ? (
                    <p className="lan-empty">{t("history_empty")}</p>
                ) : (
                    <table className="lan-table">
                        <thead>
                            <tr>
                                <th>{t("col_time")}</th>
                                <th>{t("col_direction")}</th>
                                <th>{t("col_peer")}</th>
                                <th>{t("col_file")}</th>
                                <th className="is-size">{t("col_size")}</th>
                                <th>{t("col_status")}</th>
                                <th className="is-action" />
                            </tr>
                        </thead>
                        <tbody>
                            {batches.map((batch) =>
                                batch.rows.map((row, index) => (
                                    <tr key={row.id}>
                                        <td>{formatStamp(row.createdAt, lang)}</td>
                                        <td>
                                            <LanPill
                                                tone={row.direction === "in" ? "accent" : "muted"}
                                            >
                                                {row.direction === "in" ? t("dir_in") : t("dir_out")}
                                            </LanPill>
                                        </td>
                                        <td>{row.peerName}</td>
                                        <td className="is-file" title={row.error || row.fileName}>
                                            {row.fileName}
                                            {batch.rows.length > 1 && index === 0
                                                ? ` · ${t("batch_row", { count: batch.rows.length })}`
                                                : ""}
                                        </td>
                                        <td className="is-size">{formatBytes(row.totalSize)}</td>
                                        <td>
                                            <span
                                                className={
                                                    row.status === "done"
                                                        ? "lan-state is-ok"
                                                        : "lan-state is-bad"
                                                }
                                            >
                                                {row.status === "done"
                                                    ? t("status_done")
                                                    : t("status_failed")}
                                            </span>
                                        </td>
                                        <td className="is-action">
                                            {row.status === "done" ? null : (
                                                // One button for the whole batch, on its
                                                // first row — the rest of the group retries
                                                // with it.
                                                index === 0 ? (
                                                    <LanButton onClick={() => retry(batch.group)}>
                                                        {t("retry")}
                                                    </LanButton>
                                                ) : null
                                            )}
                                        </td>
                                    </tr>
                                )),
                            )}
                        </tbody>
                    </table>
                )}
            </LanCard>

            <ConfirmDialog
                open={confirming}
                title={t("clear_confirm_title")}
                message={t("clear_confirm_body")}
                confirmLabel={t("clear_records")}
                cancelLabel={t("cancel")}
                danger
                onConfirm={clear}
                onClose={() => setConfirming(false)}
            />
        </>
    );
}
