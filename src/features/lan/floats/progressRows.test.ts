import { describe, expect, test } from "vitest";
import {
    mergeProgressRows,
    removeBatchedRows,
    running,
    settleGroup,
    type ProgressEvent,
    type ProgressRow,
} from "./progressRows";

const event = (over: Partial<ProgressEvent>): ProgressEvent => ({
    group: "g1",
    direction: "out",
    peerName: "书房的MacBook",
    fileName: "",
    uploadId: "",
    sentBytes: 0,
    totalBytes: 0,
    status: "preparing",
    timestamp: 1000,
    ...over,
});

const merge = (rows: ProgressRow[], events: ProgressEvent[]) =>
    events.reduce(mergeProgressRows, rows);

const file = (over: Partial<ProgressEvent>): ProgressEvent =>
    event({ fileName: "a.jpg", uploadId: "u1", ...over });

describe("progress rows", () => {
    test("chunks of one file update one row", () => {
        const rows = merge([], [
            file({ status: "active", sentBytes: 512, totalBytes: 2048, timestamp: 1100 }),
            file({ status: "active", sentBytes: 1024, totalBytes: 2048, timestamp: 1200 }),
            file({ status: "done", sentBytes: 2048, totalBytes: 2048, timestamp: 1300 }),
        ]);

        expect(rows).toHaveLength(1);
        expect(rows[0].status).toBe("done");
        expect(rows[0].sent).toBe(2048);
    });

    test("the invite placeholder gives way to the first real file", () => {
        // The placeholder is what used to stay "preparing" forever, keeping the
        // card on screen after the transfer had finished.
        const rows = merge([], [
            event({ status: "preparing", timestamp: 1000 }),
            file({ status: "done", sentBytes: 10, totalBytes: 10, timestamp: 1100 }),
        ]);

        expect(rows).toHaveLength(1);
        expect(rows[0].key).toBe("g1:u1");
        expect(rows.every((row) => !running(row.status))).toBe(true);
    });

    test("an invite is not re-added after the group has rows", () => {
        const withFile = merge([], [
            file({ status: "active", sentBytes: 1, totalBytes: 10 }),
        ]);
        const again = mergeProgressRows(
            withFile,
            event({ status: "preparing", timestamp: 2000 }),
        );

        expect(again).toBe(withFile);
    });

    test("a refused invite stays, because 重试 lives on that row", () => {
        const rows = merge([], [
            event({ status: "failed", error: "对方没有接受这次传输", timestamp: 1000 }),
        ]);

        expect(rows).toHaveLength(1);
        expect(rows[0].status).toBe("failed");
        expect(rows[0].error).toBe("对方没有接受这次传输");
    });

    test("one group's file events leave another group's rows alone", () => {
        const rows = merge([], [
            event({ group: "g2", status: "preparing", timestamp: 1000 }),
            file({ group: "g1", status: "active", sentBytes: 1, totalBytes: 10 }),
        ]);

        expect(rows.map((row) => row.group)).toEqual(["g2", "g1"]);
    });

    test("the end of a group settles a row whose last event was lost", () => {
        const rows = merge([], [
            file({ group: "g1", status: "active", sentBytes: 4, totalBytes: 10 }),
            file({ group: "g2", uploadId: "u2", status: "active", sentBytes: 1, totalBytes: 9 }),
        ]);
        const settled = settleGroup(rows, "g1", "done");

        expect(settled[0].status).toBe("done");
        expect(settled[0].sent).toBe(10);
        expect(settled[1].status).toBe("active");
        expect(running(settled[0].status)).toBe(false);
    });

    test("a failed group does not pretend the running file succeeded", () => {
        const rows = merge([], [
            file({ status: "active", sentBytes: 4, totalBytes: 10 }),
        ]);
        const settled = settleGroup(rows, "g1", "failed");

        expect(settled[0].status).toBe("failed");
        expect(settled[0].sent).toBe(4);
        // Failures stay until the user reads them; that is the card's rule.
        expect(settled.some((row) => row.status === "failed")).toBe(true);
    });

    test("a retry drops the previous run but not events that came after it", () => {
        const row = (over: Partial<ProgressRow>): ProgressRow => ({
            key: "g1:u1",
            group: "g1",
            direction: "out",
            peerName: "书房的MacBook",
            fileName: "a.jpg",
            sent: 0,
            total: 10,
            status: "failed",
            error: "",
            at: 1000,
            ...over,
        });
        const rows = [
            row({ key: "g1:u1", at: 1000 }),
            row({ key: "g1:u2", at: 2500 }),
            row({ key: "g2:u1", group: "g2", at: 1200 }),
        ];
        const kept = removeBatchedRows(rows, "g1", 2000);

        expect(kept.map((item) => item.key)).toEqual(["g1:u2", "g2:u1"]);
    });
});
