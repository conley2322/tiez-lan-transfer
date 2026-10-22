// 文件传输 — where received files land, whether they ask first, and how a
// running transfer shows itself on the desktop.
//
// Every row here writes a setting Rust re-reads per event, not per launch:
// 「保存位置」 is looked up when an upload is opened, 「接收方式」 and 「超时」
// when an invite arrives, 「通知」 when the last file of a batch lands, and the
// three progress rows when the next card is summoned. Nothing needs a restart.

import { useEffect, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { FolderOpen } from "lucide-react";
import * as api from "../api";
import {
    LanButton,
    LanCard,
    LanPill,
    LanRange,
    LanRow,
    LanSegment,
    LanSelect,
    LanSwitch,
} from "../controls";
import type { SettingsCtx } from "../LanSettingsWindow";

type Mode = "auto" | "confirm";

/** The three durations the slide card can realistically be read inside; Rust
 *  clamps to 15–120 anyway, so a hand-edited database row is still honoured. */
const TIMEOUTS = ["30", "60", "120"] as const;

const POSITIONS = ["bottom-right", "bottom-left", "top-right"] as const;
type Position = (typeof POSITIONS)[number];

export default function TransferPage({ ctx }: { ctx: SettingsCtx }) {
    const { t, settings, save, float } = ctx;
    const [folder, setFolder] = useState("");

    // The default is 下载/TieZ, which only Rust knows — an empty settings row
    // does not mean "nowhere".
    useEffect(() => {
        api.getReceivePath()
            .then(setFolder)
            .catch(() => {});
    }, []);

    const confirmMode = settings["peer_confirm_receive"] === "true";
    const timeout = settings["peer_receive_timeout"] ?? "60";
    const notify = settings["peer_notify_received"] !== "false";
    const hold = float?.holdSeconds ?? 3;
    const position = (float?.position ?? "bottom-right") as Position;
    const showProgress = float?.progressEnabled ?? true;

    const pickFolder = async () => {
        try {
            const picked = await open({
                directory: true,
                title: t("pick_folder_title"),
                defaultPath: folder || undefined,
            });
            if (typeof picked === "string" && picked.length > 0) {
                save("file_transfer_path", picked);
                setFolder(picked);
            }
        } catch {
            // A cancelled or unsupported picker leaves the old path in place.
        }
    };

    return (
        <>
            <h2 className="lan-page-title">{t("transfer_title")}</h2>

            <LanCard title={t("receive_group")}>
                <LanRow label={t("save_location")} hint={t("save_location_hint")} stack>
                    <div className="lan-path-row">
                        <span className="lan-path" title={folder}>
                            {folder || "…"}
                        </span>
                        <LanButton onClick={pickFolder}>{t("change")}</LanButton>
                        <LanButton
                            onClick={() => {
                                api.openTransferFolder().catch(() => {});
                            }}
                            title={folder}
                        >
                            <FolderOpen size={14} />
                            {t("open_folder")}
                        </LanButton>
                    </div>
                </LanRow>
                <LanRow label={t("receive_mode")} hint={t("receive_mode_hint")}>
                    <LanSegment<Mode>
                        value={confirmMode ? "confirm" : "auto"}
                        ariaLabel={t("receive_mode")}
                        options={[
                            { value: "auto", label: t("mode_auto") },
                            { value: "confirm", label: t("mode_confirm") },
                        ]}
                        onChange={(next) =>
                            save("peer_confirm_receive", next === "confirm" ? "true" : "false")
                        }
                    />
                </LanRow>
                {confirmMode ? (
                    <LanRow label={t("mode_confirm_timeout")}>
                        <LanSelect<string>
                            value={timeout}
                            ariaLabel={t("mode_confirm_timeout")}
                            options={TIMEOUTS.map((sec) => ({
                                value: sec,
                                label: t("seconds", { sec: Number(sec) }),
                            }))}
                            onChange={(next) => save("peer_receive_timeout", next)}
                        />
                    </LanRow>
                ) : null}
                <LanRow label={t("notify_received")}>
                    <LanSwitch
                        checked={notify}
                        onChange={(next) => save("peer_notify_received", String(next))}
                        label={t("notify_received")}
                    />
                </LanRow>
            </LanCard>

            <LanCard title={t("progress_group")}>
                <LanRow label={t("show_progress")}>
                    <LanSwitch
                        checked={showProgress}
                        onChange={(next) => save("peer_progress_float", String(next))}
                        label={t("show_progress")}
                    />
                </LanRow>
                {showProgress ? (
                    <>
                        <LanRow label={t("position")}>
                            <LanSegment<Position>
                                value={position}
                                ariaLabel={t("position")}
                                options={POSITIONS.map((id) => ({
                                    value: id,
                                    label:
                                        id === "bottom-right"
                                            ? t("pos_bottom_right")
                                            : id === "bottom-left"
                                              ? t("pos_bottom_left")
                                              : t("pos_top_right"),
                                }))}
                                onChange={(next) => save("peer_progress_position", next)}
                            />
                        </LanRow>
                        <LanRow label={t("hold")} hint={t("hold_hint")}>
                            <div className="lan-range-row">
                                <LanRange
                                    value={hold}
                                    min={1}
                                    max={30}
                                    ariaLabel={t("hold")}
                                    onChange={(next) => save("peer_progress_hold", String(next))}
                                />
                                <span className="lan-range-value">{t("seconds", { sec: hold })}</span>
                            </div>
                        </LanRow>
                    </>
                ) : null}
                {/* Not a switch: the digest is computed on both ends and the
                    received file is dropped when they differ, so there is no
                    state of the app where it is off. A toggle here would only
                    be able to lie. */}
                <LanRow label={t("checksum")} hint={t("checksum_hint")}>
                    <LanPill tone="ok">{t("always_on")}</LanPill>
                </LanRow>
            </LanCard>

            <LanCard tone="note" title={t("send_group")}>
                <p className="lan-note-body">{t("send_hint")}</p>
                <p className="lan-note-body">{t("max_targets")}</p>
            </LanCard>
        </>
    );
}
