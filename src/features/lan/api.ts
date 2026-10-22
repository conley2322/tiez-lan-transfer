// Everything the LAN windows send to Rust, in one place.
//
// The types mirror the `…View` structs in src-tauri/src/services/peer_transfer
// and the transfer-log row. They are hand-matched rather than generated, so a
// renamed field shows up as `undefined` at runtime — which is why the pages read
// through these helpers instead of `invoke` directly, and why
// src/contracts/tauriCommands.test.ts parses this file for its literal calls.

import { invoke } from "@tauri-apps/api/core";

// ── shapes ───────────────────────────────────────────────────

export type Peer = {
    deviceId: string;
    deviceName: string;
    nickname: string;
    platform: string;
    ip: string;
    port: number;
    trusted: boolean;
    avatarColor: string;
    lastSeen: number;
    favorite: boolean;
    online: boolean;
};

export type SelfIdentity = {
    deviceId: string;
    deviceName: string;
    nickname: string;
    platform: string;
    avatarColor: string;
};

export type PairRequest = {
    deviceId: string;
    deviceName: string;
    nickname: string;
    platform: string;
    ip: string;
    verifyWord: string;
    expiresIn: number;
};

export type PairRequests = { inbound: PairRequest[]; outbound: PairRequest[] };

export type PairResult = {
    status: "paired" | "pending" | string;
    deviceId: string;
    deviceName: string;
    verifyWord: string;
    expiresIn: number;
};

export type ReceiveRequest = {
    group: string;
    deviceId: string;
    deviceName: string;
    platform: string;
    files: Array<{ name: string; size: number }>;
    totalSize: number;
    expiresIn: number;
};

export type TransferRow = {
    id: number;
    groupId: string;
    direction: "in" | "out" | string;
    peerDeviceId: string;
    peerName: string;
    fileName: string;
    totalSize: number;
    status: "done" | "failed" | string;
    error: string;
    createdAt: number;
};

export type TransferEvent = {
    group: string;
    direction: "in" | "out" | string;
    peerDeviceId: string;
    peerName: string;
    fileName: string;
    uploadId: string;
    sentBytes: number;
    totalBytes: number;
    status: "preparing" | "active" | "done" | "failed" | string;
    error?: string;
    batchId?: string;
    batchName?: string;
    batchIndex?: number;
    batchTotal?: number;
    timestamp: number;
};

export type SelfCheck = {
    ready: boolean;
    // `label` and `detail` are Chinese sentences from Rust on purpose: they are
    // facts about this machine, not locale strings, and the page prints them.
    checks: Array<{ key: string; label: string; ok: boolean; detail: string }>;
};

export type CornerState = {
    enabled: boolean;
    side: string;
    opacity: number;
    favoritesOnly: boolean;
};

/** What Rust actually wrote into HKCU for the right-click submenu. `entries` is
 *  read back from the registry, so the page shows the menu Explorer will show —
 *  not the roster the page hopes the registry was rebuilt from. */
export type WinMenuState = {
    enabled: boolean;
    entries: string[];
    exePath: string;
};

export type FloatState = {
    position: string;
    holdSeconds: number;
    progressEnabled: boolean;
};

export type NearbyPeer = {
    deviceId: string;
    deviceName: string;
    ip: string;
    port: number;
};

export type ServerStatus = {
    enabled: boolean;
    port: number;
    ip: string;
    access_token: string;
    lan_key: string;
};

// ── roster & identity ────────────────────────────────────────

export const listPeers = () => invoke<Peer[]>("list_peers");
export const listNearby = () => invoke<NearbyPeer[]>("list_nearby_peers");
export const getIdentity = () => invoke<SelfIdentity>("get_peer_identity");
export const setSelfNickname = (nickname: string) =>
    invoke<void>("set_peer_nickname", { nickname });
export const setSelfColor = (avatarColor: string) =>
    invoke<void>("set_self_avatar_color", { color: avatarColor });
export const setPeerProfile = (deviceId: string, nickname: string, avatarColor: string) =>
    invoke<void>("update_peer_profile", { deviceId, nickname, avatarColor });
export const setPeerFavorite = (deviceId: string, favorite: boolean) =>
    invoke<void>("set_peer_favorite", { deviceId, favorite });
export const removePeer = (deviceId: string) => invoke<void>("remove_peer", { deviceId });

// ── pairing ──────────────────────────────────────────────────

export const generatePairingCode = () => invoke<string>("get_peer_pairing_code");
export const pairWithCode = (code: string) => invoke<PairResult>("pair_with_code", { code });
export const listPairRequests = () => invoke<PairRequests>("list_pair_requests");
export const respondPair = (deviceId: string, accept: boolean) =>
    invoke<void>("respond_pair_request", { deviceId, accept });

// ── receiving ────────────────────────────────────────────────

export const listReceiveRequests = () => invoke<ReceiveRequest[]>("list_receive_requests");
export const respondReceive = (group: string, accept: boolean) =>
    invoke<boolean>("respond_receive_request", { group, accept });

// ── sending & log ────────────────────────────────────────────

export const sendFiles = (peerIds: string[], paths: string[], batchName: string | null) =>
    invoke<string>("send_files_to_peers", { peerIds, paths, batchName });
export const retryTransfer = (group: string) => invoke<string>("retry_peer_transfer", { group });
export const cancelTransfer = (group: string) => invoke<boolean>("cancel_peer_transfer", { group });
export const listTransfers = (limit?: number) =>
    invoke<TransferRow[]>("list_peer_transfers", { limit });
export const clearTransfers = () => invoke<number>("clear_peer_transfers");
export const runSelfCheck = () => invoke<SelfCheck>("run_network_selfcheck");

// ── windows & settings ───────────────────────────────────────

export const getSettings = () => invoke<Record<string, string>>("get_settings");
export const saveSetting = (key: string, value: string) =>
    invoke<void>("save_setting", { key, value });
export const getServerStatus = () => invoke<ServerStatus>("get_file_server_status");
export const toggleServer = (enabled: boolean) =>
    invoke<string>("toggle_file_server", { enabled, port: null });
export const openTransferFolder = () => invoke<void>("open_file_transfer_folder");
export const getReceivePath = () => invoke<string>("get_active_file_transfer_path");
export const getDataPath = () => invoke<string>("get_data_path");
export const openDataFolder = () => invoke<void>("open_data_folder");
export const isReceivingPaused = () => invoke<boolean>("get_pause_receiving");
export const setReceivingPaused = (paused: boolean) => invoke<void>("set_pause_receiving", { paused });
export const isAutostartEnabled = () => invoke<boolean>("is_autostart_enabled");
export const setAutostart = (enabled: boolean) => invoke<void>("toggle_autostart", { enabled });
export const quitApp = () => invoke<void>("quit");

export const getCornerState = () => invoke<CornerState>("get_corner_box_state");
export const setCornerEnabled = (enabled: boolean) =>
    invoke<void>("set_corner_box_enabled", { enabled });
export const setCornerSide = (side: string) => invoke<void>("set_corner_box_side", { side });
export const setCornerOpacity = (opacity: number) =>
    invoke<void>("set_corner_box_opacity", { opacity });
export const setCornerFavoritesOnly = (favoritesOnly: boolean) =>
    invoke<void>("set_corner_box_favorites_only", { favoritesOnly });

export const getWinMenuState = () => invoke<WinMenuState>("get_win_menu_state");
export const setWinMenuEnabled = (enabled: boolean) =>
    invoke<void>("set_win_menu_enabled", { enabled });

export const getFloatState = () => invoke<FloatState>("get_peer_float_state");
export const hideFloat = (label: string) => invoke<void>("hide_peer_float", { label });
export const syncFloatLayout = () => invoke<void>("sync_peer_float_layout");
