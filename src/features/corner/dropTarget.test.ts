import { describe, expect, test } from "vitest";
import {
    droppablePeers,
    peerAtPoint,
    peerColor,
    peerInitial,
    peerLabel,
    visiblePeers,
    MAX_CORNER_ROWS,
    type CornerPeer,
    type RowRect,
} from "./dropTarget";

const ROWS: RowRect[] = [
    { id: "a", left: 12, top: 38, right: 178, bottom: 78 },
    { id: "b", left: 12, top: 78, right: 178, bottom: 118 },
];

const peer = (over: Partial<CornerPeer>): CornerPeer => ({
    deviceId: "device-0001",
    deviceName: "Conley 的 Mac",
    nickname: "",
    platform: "macos",
    online: true,
    trusted: true,
    avatarColor: "#0a84ff",
    favorite: false,
    ...over,
});

describe("corner drop target", () => {
    test("a point inside a row is that peer", () => {
        expect(peerAtPoint(ROWS, 90, 50)).toBe("a");
        expect(peerAtPoint(ROWS, 20, 100)).toBe("b");
    });

    test("the seams between rows are nobody's", () => {
        // A row ends where the next begins: one pixel of overlap would hand a
        // drop to the wrong machine.
        expect(peerAtPoint(ROWS, 90, 78)).toBe("b");
        expect(peerAtPoint(ROWS, 90, 77)).toBe("a");
    });

    test("outside the rows cancels instead of guessing", () => {
        expect(peerAtPoint(ROWS, 90, 20)).toBeNull(); // the hint line
        expect(peerAtPoint(ROWS, 4, 50)).toBeNull(); // the panel padding
        expect(peerAtPoint(ROWS, 178, 50)).toBeNull(); // right edge is exclusive
        expect(peerAtPoint(ROWS, 90, 400)).toBeNull();
        expect(peerAtPoint([], 90, 50)).toBeNull();
    });

    test("only paired machines are offered", () => {
        const peers = [peer({ deviceId: "a" }), peer({ deviceId: "b", trusted: false })];
        expect(droppablePeers(peers).map((p) => p.deviceId)).toEqual(["a"]);
    });

    test("an awake machine is listed first", () => {
        const peers = [
            peer({ deviceId: "off", online: false }),
            peer({ deviceId: "on", online: true }),
        ];
        expect(droppablePeers(peers).map((p) => p.deviceId)).toEqual(["on", "off"]);
    });

    test("a starred machine outranks an awake one", () => {
        // The same order 设置 and the transfer list use: the machine the user
        // cares about is the first row they drag onto, asleep or not.
        const peers = [
            peer({ deviceId: "loud", online: true }),
            peer({ deviceId: "star", online: false, favorite: true }),
        ];
        expect(droppablePeers(peers).map((p) => p.deviceId)).toEqual(["star", "loud"]);
    });

    test("the box shows at most eight rows", () => {
        const peers = Array.from({ length: 12 }, (_, i) => peer({ deviceId: `p${i}` }));
        // Rust clamps the window height to the same number; more rows would be
        // drawn off-screen and still be a drop target.
        expect(visiblePeers(peers)).toHaveLength(MAX_CORNER_ROWS);
    });

    test("the name is what the user typed", () => {
        expect(peerLabel(peer({ nickname: "书房", deviceName: "Conley 的 Mac" }))).toBe("书房");
        expect(peerLabel(peer({ nickname: "   ", deviceName: "MacBook Pro" }))).toBe("MacBook Pro");
        expect(peerLabel(peer({ nickname: "", deviceName: "", deviceId: "a3f9c2d4e5" }))).toBe("a3f9c2d4");
    });

    test("one character, even when it is not one code unit", () => {
        expect(peerInitial(peer({ nickname: "阿强" }))).toBe("阿");
        expect(peerInitial(peer({ nickname: "mac" }))).toBe("M");
        // An emoji is a surrogate pair; slicing by code units would draw a box.
        expect(peerInitial(peer({ nickname: "🐱的电脑" }))).toBe("🐱");
        expect(peerInitial(peer({ nickname: "", deviceName: "", deviceId: "" }))).toBe("?");
    });

    test("a colour that is not a colour never reaches the style", () => {
        expect(peerColor(peer({ avatarColor: "#0a84ff" }))).toBe("#0a84ff");
        expect(peerColor(peer({ avatarColor: "#f00" }))).toBe("#f00");
        for (const bad of ["", "red", "javascript:alert(1)", "#12345", "#fff;", " url(x)"]) {
            expect(peerColor(peer({ avatarColor: bad }))).toBeNull();
        }
    });
});
