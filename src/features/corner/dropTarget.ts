//! Pure helpers behind the corner drop box: which row the file is hovering, and
//! who is worth showing at all.

/** One avatar row, as it sits in the window's own pixel space. */
export type RowRect = {
    id: string;
    left: number;
    top: number;
    right: number;
    bottom: number;
};

/**
 * The peer under a drag position, or null.
 *
 * The position is window-local and so is the rect (`getBoundingClientRect` on a
 * window that is exactly its content), which is what makes the two comparable.
 * Null means the file is over the panel's padding or the hint line: dropping
 * there cancels rather than sending to whoever happens to be nearest.
 */
export function peerAtPoint(rows: RowRect[], x: number, y: number): string | null {
    const hit = rows.find(
        (row) => x >= row.left && x < row.right && y >= row.top && y < row.bottom,
    );
    return hit ? hit.id : null;
}

export type CornerPeer = {
    deviceId: string;
    deviceName: string;
    nickname: string;
    platform: string;
    online: boolean;
    trusted: boolean;
    avatarColor: string;
    favorite: boolean;
};

/** What the three display helpers below actually read. `CornerPeer` is wider,
 *  but the settings pages have their own views from Rust, and an avatar should
 *  look the same in both. */
export type PeerDisplay = Pick<CornerPeer, "deviceId" | "deviceName" | "nickname" | "avatarColor">;

/** The box can only send to a machine this one has paired with. */
export function droppablePeers(peers: CornerPeer[]): CornerPeer[] {
    const sendable = peers.filter((peer) => peer.trusted);
    // ⭐ first, then the one a user is standing next to: the same order every
    // list in the app uses, so the row a file lands on is the row they meant.
    return [...sendable].sort(
        (a, b) => Number(b.favorite) - Number(a.favorite) || Number(b.online) - Number(a.online),
    );
}

/** What the avatar shows: the nickname the user typed, else the device name. */
export function peerLabel(peer: PeerDisplay): string {
    const nickname = peer.nickname.trim();
    if (nickname) return nickname;
    const device = peer.deviceName.trim();
    if (device) return device;
    return peer.deviceId.slice(0, 8);
}

export function peerInitial(peer: PeerDisplay): string {
    const label = peerLabel(peer);
    // Counted in characters, not code units: an emoji or a CJK name is one
    // glyph, and the first half of a surrogate pair is a broken box.
    const first = Array.from(label)[0];
    return first ? first.toUpperCase() : "?";
}

/**
 * `#rgb`/`#rrggbb` only — Rust normalizes it the same way before storing, and
 * anything else must not reach an inline style. Null keeps the CSS token color.
 */
export function peerColor(peer: PeerDisplay): string | null {
    return /^#(?:[0-9a-fA-F]{3}|[0-9a-fA-F]{6})$/.test(peer.avatarColor)
        ? peer.avatarColor
        : null;
}

/** The box is only as tall as the rows it shows; the rest wait in 设置. */
export const MAX_CORNER_ROWS = 8;

export function visiblePeers(peers: CornerPeer[]): CornerPeer[] {
    return droppablePeers(peers).slice(0, MAX_CORNER_ROWS);
}
