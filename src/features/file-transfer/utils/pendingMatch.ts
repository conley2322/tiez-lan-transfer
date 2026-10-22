import type { FileTransferMessage } from "../types";

/** File name of a finished transfer message: from its path, else its download link. */
export const messageFileName = (message: FileTransferMessage): string => {
    const fromPath = message.file_path?.split(/[\\/]/).pop();
    if (fromPath) return fromPath;
    const index = message.content.indexOf("?name=");
    if (index === -1) return "";
    try {
        return decodeURIComponent(message.content.slice(index + "?name=".length));
    } catch {
        return message.content.slice(index + "?name=".length);
    }
};

const isFileMessage = (message: FileTransferMessage) =>
    message.msg_type === "file" || message.msg_type === "image" || message.msg_type === "video";

/**
 * The pending bubbles no finished message stands for yet. A finished message
 * matches by upload id or the exact file name (a substring let "aa.png"
 * finish "a.png"); a file the computer saved under another name ("a (2).png")
 * matches the first file from the same phone that arrived after it started.
 */
export const unfinishedPending = (
    pending: readonly FileTransferMessage[],
    messages: readonly FileTransferMessage[]
): FileTransferMessage[] => {
    const used = new Set<number>();
    const take = (found: FileTransferMessage | undefined) => {
        if (!found) return false;
        used.add(found.id);
        return true;
    };
    const exact = pending.filter((p) => !take(messages.find((m) =>
        !used.has(m.id)
        && ((p._uploadId && m.content.includes(p._uploadId))
            || (p._fileName && messageFileName(m) === p._fileName))
    )));
    return exact.filter((p) => p._error || !p.sender_id || !take(messages.find((m) =>
        !used.has(m.id)
        && isFileMessage(m)
        && m.sender_id === p.sender_id
        && m.timestamp >= p.timestamp
    )));
};
