import { Monitor, Smartphone, Tablet } from "lucide-react";
import type { FileTransferMessage } from "../types";

const TONES = 6;

/** The same device always gets the same color, so several connected devices stay apart. */
function toneFor(key: string) {
    let hash = 0;
    for (const ch of key) hash = (hash * 31 + (ch.codePointAt(0) ?? 0)) >>> 0;
    return hash % TONES;
}

function DeviceIcon({ name }: { name: string }) {
    const lower = name.toLowerCase();
    if (/ipad|tablet|平板/.test(lower)) return <Tablet size={14} />;
    if (/mac|windows|linux|desktop|电脑|電腦/.test(lower)) return <Monitor size={14} />;
    return <Smartphone size={14} />;
}

/**
 * The device an incoming message came from: its kind as an icon, its identity
 * as a color. Device names ("Mobile", "iPhone") repeat across devices, so
 * initials would not tell them apart; the name itself is shown above the bubble.
 */
export const TransferPeerAvatar = ({ message }: { message: FileTransferMessage }) => (
    <div
        className="wt-avatar"
        data-tone={toneFor(message.sender_id || message.sender_name || "")}
        aria-hidden="true"
    >
        <DeviceIcon name={message.sender_name || ""} />
    </div>
);
