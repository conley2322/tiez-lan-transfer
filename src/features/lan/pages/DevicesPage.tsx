// 设备与用户 — who this machine is, who it knows, and how a new machine gets in.
//
// Two ways in, exactly as LocalSend does it: the beacon (自动发现) lists machines
// on the subnet, and the pairing code carries everything a machine outside that
// list needs (address, port, rotating token, and the public half of its key).

import { useCallback, useEffect, useState } from "react";
import { Copy, MoreHorizontal, Plus, RefreshCw, Trash2 } from "lucide-react";
import * as api from "../api";
import { platformBadge, relativeTime } from "../format";
import {
    LanAvatar,
    LanButton,
    LanCard,
    LanColorRow,
    LanPill,
    LanRow,
    LanStar,
    LanStatus,
    LanSwitch,
} from "../controls";
import ConfirmDialog from "../../../shared/components/ConfirmDialog";
import type { SettingsCtx } from "../LanSettingsWindow";

type Editor = { deviceId: string; nickname: string; avatarColor: string; self: boolean };

const shortId = (id: string) => id.slice(0, 8);

export default function DevicesPage({ ctx }: { ctx: SettingsCtx }) {
    const { t, lang, settings, save, peers, refreshPeers, identity, refreshIdentity, refreshCorner } = ctx;
    const [nearby, setNearby] = useState<api.NearbyPeer[]>([]);
    const [requests, setRequests] = useState<api.PairRequests>({ inbound: [], outbound: [] });
    const [code, setCode] = useState<string | null>(null);
    const [codeCopied, setCodeCopied] = useState(false);
    const [pasteOpen, setPasteOpen] = useState(false);
    const [pasteValue, setPasteValue] = useState("");
    const [pasteResult, setPasteResult] = useState<string | null>(null);
    const [editor, setEditor] = useState<Editor | null>(null);
    const [menuFor, setMenuFor] = useState<string | null>(null);
    const [removeTarget, setRemoveTarget] = useState<api.Peer | null>(null);
    const [check, setCheck] = useState<api.SelfCheck | null>(null);
    const [checking, setChecking] = useState(false);
    const [notice, setNotice] = useState<string | null>(null);

    const discoveryOn = settings["peer_discovery_enabled"] !== "false";

    const refreshNearby = useCallback(() => {
        api.listNearby()
            .then(setNearby)
            .catch(() => {});
    }, []);

    const refreshRequests = useCallback(() => {
        api.listPairRequests()
            .then(setRequests)
            .catch(() => {});
    }, []);

    // Beacons expire on their own (15s), and a pending pairing answers whenever
    // the other user gets around to it, so this page keeps its own clock.
    useEffect(() => {
        refreshNearby();
        refreshRequests();
        const timer = window.setInterval(() => {
            refreshNearby();
            refreshRequests();
        }, 4000);
        return () => window.clearInterval(timer);
    }, [refreshNearby, refreshRequests]);

    const generate = () => {
        setNotice(null);
        api.generatePairingCode()
            .then(setCode)
            .catch((error) => setNotice(t("pairing_failed", { reason: String(error) })));
    };

    const copyCode = () => {
        if (!code) return;
        navigator.clipboard
            .writeText(code)
            .then(() => {
                setCodeCopied(true);
                window.setTimeout(() => setCodeCopied(false), 1800);
            })
            .catch(() => {});
    };

    const submitPaste = () => {
        const value = pasteValue.trim();
        if (!value) return;
        setPasteResult(null);
        api.pairWithCode(value)
            .then((result) => {
                if (result.status === "paired") {
                    setPasteResult(t("pairing_done", { name: result.deviceName }));
                    setPasteOpen(false);
                    setPasteValue("");
                } else {
                    setPasteResult(t("pairing_waiting", { sec: result.expiresIn }));
                }
                refreshRequests();
                refreshPeers();
            })
            .catch((error) => setPasteResult(t("pairing_failed", { reason: String(error) })));
    };

    const answer = (deviceId: string, accept: boolean) => {
        api.respondPair(deviceId, accept)
            .catch((error) => setNotice(String(error)))
            .finally(() => {
                refreshRequests();
                refreshPeers();
            });
    };

    const star = (deviceId: string, value: boolean) => {
        api.setPeerFavorite(deviceId, value)
            .then(() => {
                refreshPeers();
                // The corner box lists favorites only when the switch is on, and
                // Rust decides that list — so ask it again rather than sorting here.
                refreshCorner();
            })
            .catch((error) => setNotice(String(error)));
    };

    const doRemove = (peer: api.Peer) => {
        api.removePeer(peer.deviceId)
            .catch((error) => setNotice(String(error)))
            .finally(() => {
                setRemoveTarget(null);
                refreshPeers();
                refreshCorner();
            });
    };

    const runCheck = () => {
        setChecking(true);
        setCheck(null);
        api.runSelfCheck()
            .then(setCheck)
            .catch(() => {})
            .finally(() => setChecking(false));
    };

    const saveEditor = (nickname: string, avatarColor: string) => {
        if (!editor) return;
        const request = editor.self
            ? api.setSelfNickname(nickname).then(() => api.setSelfColor(avatarColor))
            : api.setPeerProfile(editor.deviceId, nickname, avatarColor);
        request
            .then(() => {
                if (editor.self) refreshIdentity();
                else {
                    refreshPeers();
                    refreshCorner();
                }
                setEditor(null);
            })
            .catch((error) => setNotice(String(error)));
    };

    const paired = peers.filter((peer) => peer.trusted);
    const pendingCount = peers.length - paired.length + requests.inbound.length;
    const inboundIds = new Set(requests.inbound.map((item) => item.deviceId));

    return (
        <>
            <h2 className="lan-page-title">{t("tab_devices")}</h2>

            <LanCard title={t("my_device")}>
                <div className="lan-peer">
                    <LanAvatar
                        peer={{
                            nickname: identity?.nickname ?? "",
                            deviceName: identity?.deviceName ?? "",
                            deviceId: identity?.deviceId ?? "",
                            avatarColor: identity?.avatarColor ?? "",
                        }}
                    />
                    <div className="lan-peer-info">
                        <div className="lan-peer-name">
                            {identity?.nickname || identity?.deviceName || "—"}
                        </div>
                        <div className="lan-peer-meta">
                            {platformBadge(identity?.platform ?? "")} · {t("device_id_label")}{" "}
                            {shortId(identity?.deviceId ?? "")}
                        </div>
                        <div className="lan-peer-meta">{t("my_device_hint")}</div>
                    </div>
                    <LanButton
                        onClick={() =>
                            setEditor({
                                deviceId: identity?.deviceId ?? "",
                                nickname: identity?.nickname ?? "",
                                avatarColor: identity?.avatarColor ?? "",
                                self: true,
                            })
                        }
                    >
                        {t("act_rename")}
                    </LanButton>
                </div>
            </LanCard>

            <LanCard>
                <div className="lan-card-head">
                    <h3 className="lan-card-title">{t("users_title")}</h3>
                    <div className="lan-card-actions">
                        <LanButton variant="primary" onClick={generate}>
                            <Plus size={14} />
                            {t("gen_code")}
                        </LanButton>
                        <LanButton onClick={() => setPasteOpen(true)}>{t("paste_code")}</LanButton>
                    </div>
                </div>
                <p className="lan-card-sub">
                    {t("users_count", { paired: paired.length, pending: pendingCount })}
                </p>

                {code ? (
                    <div className="lan-code-box">
                        <div className="lan-code-title">{t("code_yours")}</div>
                        <textarea className="lan-code" readOnly value={code} rows={3} />
                        <div className="lan-code-hint">{t("code_yours_hint")}</div>
                        <LanButton onClick={copyCode}>
                            <Copy size={14} />
                            {codeCopied ? t("copied") : t("copy")}
                        </LanButton>
                    </div>
                ) : null}

                {pasteOpen ? (
                    <div className="lan-code-box">
                        <div className="lan-code-title">{t("code_peer_title")}</div>
                        <textarea
                            className="lan-code"
                            rows={3}
                            placeholder={t("code_placeholder")}
                            value={pasteValue}
                            onChange={(event) => setPasteValue(event.target.value)}
                        />
                        <div className="lan-code-hint">{t("code_peer_hint")}</div>
                        <LanButton variant="primary" onClick={submitPaste} disabled={!pasteValue.trim()}>
                            {t("pair_send")}
                        </LanButton>
                        {pasteResult ? <p className="lan-inline-result">{pasteResult}</p> : null}
                    </div>
                ) : null}

                {peers.length === 0 && requests.inbound.length === 0 ? (
                    <p className="lan-empty">
                        {t("users_empty")}
                        {t("users_empty_action")}
                    </p>
                ) : null}

                {peers.map((peer) => {
                    const pending = !peer.trusted || inboundIds.has(peer.deviceId);
                    return (
                        <div className="lan-peer" key={peer.deviceId}>
                            <LanAvatar peer={peer} />
                            <div className="lan-peer-info">
                                <div className="lan-peer-name">
                                    {peer.nickname || peer.deviceName}
                                    <LanStar
                                        favorite={peer.favorite}
                                        label={t("favorite")}
                                        onToggle={(value) => star(peer.deviceId, value)}
                                    />
                                    {pending ? <LanPill tone="warn">{t("pending_pair")}</LanPill> : null}
                                </div>
                                <div className="lan-peer-meta">
                                    {platformBadge(peer.platform)} · {peer.ip} ·{" "}
                                    {peer.online
                                        ? t("online")
                                        : t("last_seen", { time: relativeTime(peer.lastSeen, lang) })}
                                </div>
                            </div>
                            <LanStatus
                                online={peer.online}
                                labels={{ on: t("online"), off: t("offline") }}
                            />
                            <div className="lan-peer-menu">
                                <button
                                    type="button"
                                    className="lan-icon-button"
                                    aria-label={t("more")}
                                    aria-haspopup="menu"
                                    aria-expanded={menuFor === peer.deviceId}
                                    onClick={() => setMenuFor(menuFor === peer.deviceId ? null : peer.deviceId)}
                                >
                                    <MoreHorizontal size={16} />
                                </button>
                                {menuFor === peer.deviceId ? (
                                    <div className="lan-menu" role="menu">
                                        <button
                                            type="button"
                                            role="menuitem"
                                            onClick={() => {
                                                setEditor({
                                                    deviceId: peer.deviceId,
                                                    nickname: peer.nickname,
                                                    avatarColor: peer.avatarColor,
                                                    self: false,
                                                });
                                                setMenuFor(null);
                                            }}
                                        >
                                            {t("act_rename")}
                                        </button>
                                        <button
                                            type="button"
                                            role="menuitem"
                                            disabled={!peer.trusted}
                                            onClick={() => {
                                                setMenuFor(null);
                                                generate();
                                            }}
                                        >
                                            {t("act_repair")}
                                        </button>
                                        <button
                                            type="button"
                                            role="menuitem"
                                            className="is-danger"
                                            onClick={() => {
                                                setMenuFor(null);
                                                setRemoveTarget(peer);
                                            }}
                                        >
                                            <Trash2 size={14} />
                                            {t("act_remove")}
                                        </button>
                                    </div>
                                ) : null}
                            </div>
                        </div>
                    );
                })}
            </LanCard>

            {requests.inbound.length > 0 || requests.outbound.length > 0 ? (
                <LanCard title={t("float_pair_list")}>
                    {requests.inbound.map((request) => (
                        <div className="lan-request" key={`in-${request.deviceId}`}>
                            <div className="lan-request-info">
                                <div className="lan-peer-name">{request.nickname || request.deviceName}</div>
                                <div className="lan-peer-meta">
                                    {t("float_pair_body", {
                                        name: request.nickname || request.deviceName,
                                    })}
                                </div>
                                <div className="lan-peer-meta">
                                    {t("float_word")}
                                    <span className="lan-word">{request.verifyWord}</span>
                                    {t("float_word_hint")}
                                </div>
                            </div>
                            <div className="lan-request-actions">
                                <LanButton
                                    variant="primary"
                                    onClick={() => answer(request.deviceId, true)}
                                >
                                    {t("save")}
                                </LanButton>
                                <LanButton variant="danger" onClick={() => answer(request.deviceId, false)}>
                                    {t("float_reject")}
                                </LanButton>
                            </div>
                        </div>
                    ))}
                    {requests.outbound.map((request) => (
                        <div className="lan-request" key={`out-${request.deviceId}`}>
                            <div className="lan-request-info">
                                <div className="lan-peer-name">{request.nickname || request.deviceName}</div>
                                <div className="lan-peer-meta">
                                    {t("pairing_waiting", { sec: request.expiresIn })}
                                </div>
                                <div className="lan-peer-meta">
                                    {t("float_word")}
                                    <span className="lan-word">{request.verifyWord}</span>
                                </div>
                            </div>
                        </div>
                    ))}
                </LanCard>
            ) : null}

            <LanCard title={t("discovery_enable")}>
                <LanRow label={t("discovery_enable")} hint={t("discovery_hint")}>
                    <LanSwitch
                        checked={discoveryOn}
                        onChange={(value) => save("peer_discovery_enabled", String(value))}
                        label={t("discovery_enable")}
                    />
                </LanRow>
                {discoveryOn && nearby.length > 0 ? (
                    <div className="lan-nearby">
                        {nearby.map((item) => (
                            <div className="lan-nearby-row" key={item.deviceId}>
                                <span className="lan-nearby-name">{item.deviceName}</span>
                                <span className="lan-nearby-addr">
                                    {item.ip}:{item.port}
                                </span>
                            </div>
                        ))}
                    </div>
                ) : null}
                <div className="lan-selfcheck">
                    <LanButton onClick={runCheck} disabled={checking}>
                        <RefreshCw size={14} />
                        {checking ? t("selfcheck_running") : t("selfcheck")}
                    </LanButton>
                    <span className="lan-selfcheck-hint">{t("selfcheck_hint")}</span>
                </div>
                {check ? (
                    <div className={check.ready ? "lan-checks is-ready" : "lan-checks"}>
                        {check.checks.map((item) => (
                            <div className="lan-check" key={item.key}>
                                <span className={item.ok ? "lan-check-dot is-ok" : "lan-check-dot"} />
                                <span className="lan-check-key">{item.label}</span>
                                <span className="lan-check-detail">{item.detail}</span>
                            </div>
                        ))}
                    </div>
                ) : null}
            </LanCard>

            {editor ? (
                <div className="lan-editor">
                    <LanRow label={t("nickname_label")}>
                        <input
                            className="lan-input"
                            value={editor.nickname}
                            maxLength={24}
                            onChange={(event) =>
                                setEditor({ ...editor, nickname: event.target.value })
                            }
                        />
                    </LanRow>
                    <LanRow label={t("color_label")} stack>
                        <LanColorRow
                            value={editor.avatarColor}
                            onChange={(color) => setEditor({ ...editor, avatarColor: color })}
                            label={t("color_label")}
                        />
                    </LanRow>
                    <div className="lan-editor-actions">
                        <LanButton onClick={() => setEditor(null)}>{t("cancel")}</LanButton>
                        <LanButton variant="primary" onClick={() => saveEditor(editor.nickname.trim(), editor.avatarColor)}>
                            {t("save")}
                        </LanButton>
                    </div>
                </div>
            ) : null}

            {notice ? <p className="lan-inline-result">{notice}</p> : null}

            <ConfirmDialog
                open={removeTarget !== null}
                title={t("remove_confirm_title")}
                message={t("remove_confirm_body", {
                    name: removeTarget?.nickname || removeTarget?.deviceName || "",
                })}
                confirmLabel={t("act_remove")}
                cancelLabel={t("cancel")}
                danger
                onConfirm={() => removeTarget && doRemove(removeTarget)}
                onClose={() => setRemoveTarget(null)}
            />
        </>
    );
}
