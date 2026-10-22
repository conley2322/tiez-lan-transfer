import { expect, test } from "vitest";
import type { FileTransferMessage } from "../types";
import { unfinishedPending } from "./pendingMatch";

const msg = (id: number, fields: Partial<FileTransferMessage>): FileTransferMessage => ({
    id, direction: "in", msg_type: "file", content: "", timestamp: 100, ...fields,
});

test("a finished file only clears the bubble with its exact name", () => {
    const pending = [msg(1, { _fileName: "a.png", _preparing: true })];
    const done = [msg(10, { file_path: "/Downloads/aa.png" })];

    expect(unfinishedPending(pending, done)).toEqual(pending);
});

test("a file saved under another name clears its phone's bubble", () => {
    const pending = [msg(1, { _fileName: "a.png", sender_id: "phone", timestamp: 100, _preparing: true })];
    const done = [msg(10, { file_path: "/Downloads/a (2).png", sender_id: "phone", timestamp: 200 })];

    expect(unfinishedPending(pending, done)).toEqual([]);
});
