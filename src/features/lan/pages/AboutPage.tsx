// 关于 — what this machine is, and how to stop it.
//
// The address and port are read from the running server rather than from a
// setting: a machine that moved to another Wi-Fi has a different answer here, and
// that is the number a user needs when pairing by hand.

import { useEffect, useState } from "react";
import { getVersion } from "@tauri-apps/api/app";
import { FolderOpen, Power } from "lucide-react";
import ConfirmDialog from "../../../shared/components/ConfirmDialog";
import * as api from "../api";
import type { ServerStatus } from "../api";
import { LanButton, LanCard, LanRow } from "../controls";
import type { SettingsCtx } from "../LanSettingsWindow";

export default function AboutPage({ ctx, isMac }: { ctx: SettingsCtx; isMac: boolean }) {
    const { t, identity } = ctx;
    const [version, setVersion] = useState("");
    const [server, setServer] = useState<ServerStatus | null>(null);
    const [dataPath, setDataPath] = useState("");
    const [confirming, setConfirming] = useState(false);

    useEffect(() => {
        getVersion()
            .then(setVersion)
            .catch(() => {});
        api.getServerStatus()
            .then(setServer)
            .catch(() => {});
        api.getDataPath()
            .then(setDataPath)
            .catch(() => {});
    }, []);

    return (
        <>
            <h2 className="lan-page-title">{t("about_title")}</h2>

            <LanCard>
                <div className="lan-about-head">
                    <span className="lan-about-name">{t("about_name")}</span>
                    <p className="lan-note-body">{t("about_tagline")}</p>
                </div>
                <LanRow label={t("about_version")}>
                    <span className="lan-value">{version || "—"}</span>
                </LanRow>
                <LanRow label={t("device_id_label")}>
                    <span className="lan-value lan-value-mono">{identity?.deviceId || "—"}</span>
                </LanRow>
                <LanRow label={t("about_addr")} hint={t("about_addr_hint")}>
                    <span className="lan-value lan-value-mono">
                        {server && server.enabled ? `${server.ip}:${server.port}` : "—"}
                    </span>
                </LanRow>
                <LanRow label={t("about_data")} hint={t("about_data_hint")}>
                    <div className="lan-path-row">
                        <span className="lan-path lan-value-mono" title={dataPath}>
                            {dataPath || "—"}
                        </span>
                        <LanButton
                            onClick={() => {
                                api.openDataFolder().catch(() => {});
                            }}
                        >
                            <FolderOpen size={14} />
                            {t("open_folder")}
                        </LanButton>
                    </div>
                </LanRow>
            </LanCard>

            <LanCard tone="note">
                <p className="lan-note-body">
                    {isMac ? t("menu_mac_use") : t("menu_win_body")}
                </p>
                <LanButton variant="danger" onClick={() => setConfirming(true)}>
                    <Power size={14} />
                    {t("quit")}
                </LanButton>
            </LanCard>

            <ConfirmDialog
                open={confirming}
                title={t("quit_confirm_title")}
                message={t("quit_confirm_body")}
                confirmLabel={t("quit")}
                cancelLabel={t("cancel")}
                danger
                onConfirm={() => {
                    setConfirming(false);
                    api.quitApp().catch(() => {});
                }}
                onClose={() => setConfirming(false)}
            />
        </>
    );
}
