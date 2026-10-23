// Row bookkeeping for the progress card.
//
// Rust's unit of work is a group (one batch aimed at one machine), but an
// inbound file is its own group. The card keeps the files flat, so it has to
// fold a stream of events into rows: a file's chunks update one row, and the
// group's invite event must not leave a second row behind that no later event
// will ever finish.

export type ProgressEvent = {
    group: string;
    direction: string;
    peerName: string;
    fileName: string;
    uploadId: string;
    sentBytes: number;
    totalBytes: number;
    status: string;
    error?: string;
    timestamp: number;
};

export type ProgressRow = {
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

export const running = (status: string) => status === "preparing" || status === "active";

/** A `preparing` / invite event names no file yet: it is the whole group's row. */
const isInvite = (event: ProgressEvent) => !event.uploadId && !event.fileName;

export function rowFromEvent(event: ProgressEvent): ProgressRow {
    return {
        key: `${event.group}:${event.uploadId || event.fileName}`,
        group: event.group,
        direction: event.direction,
        peerName: event.peerName,
        fileName: event.fileName,
        // A retry re-uses the upload id, so a done row is pinned to the full
        // size rather than to the last chunk's count.
        sent: event.status === "done" ? event.totalBytes : event.sentBytes,
        total: event.totalBytes,
        status: event.status,
        error: event.error ?? "",
        at: event.timestamp,
    };
}

export function mergeProgressRows(rows: ProgressRow[], event: ProgressEvent): ProgressRow[] {
    const next = rowFromEvent(event);
    const index = rows.findIndex((row) => row.key === next.key);
    if (index >= 0) {
        const copy = [...rows];
        copy[index] = next;
        return copy;
    }
    if (!isInvite(event)) {
        // The invite row has to go as soon as the first real file arrives: it
        // stays "preparing" forever otherwise, and one running row is enough to
        // keep the card on screen after every file is done.
        return [...rows.filter((row) => row.key !== `${event.group}:`), next];
    }
    // A successful invite is only a placeholder for a group that has no rows
    // yet. A failed one still has to be shown, because 重试 lives on it.
    if (event.status === "preparing" && rows.some((row) => row.group === event.group)) {
        return rows;
    }
    return [...rows, next];
}

/** 重试 replays the whole group; rows from the previous run are stale. */
export function removeBatchedRows(
    rows: ProgressRow[],
    group: string,
    before: number,
): ProgressRow[] {
    return rows.filter((row) => row.group !== group || row.at >= before);
}

/**
 * The group is over. A row still marked running missed its terminal event —
 * without this it would keep the card up forever.
 */
export function settleGroup(rows: ProgressRow[], group: string, status: string): ProgressRow[] {
    if (status !== "done" && status !== "failed") return rows;
    let changed = false;
    const settled = rows.map((row) => {
        if (row.group !== group || !running(row.status)) return row;
        changed = true;
        return {
            ...row,
            status,
            sent: status === "done" && row.total > 0 ? row.total : row.sent,
        };
    });
    return changed ? settled : rows;
}
