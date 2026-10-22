// 常规 — the four things that describe how the app behaves, not what it sends.

import { useCallback, useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import * as api from "../api";
import { LanButton, LanCard, LanRow, LanSegment, LanSwitch } from "../controls";
import { LAN_LANGS, type LanLang } from "../lanCopy";
import type { SettingsCtx } from "../LanSettingsWindow";

export default function GeneralPage({ ctx }: { ctx: SettingsCtx }) {
    const { t, lang, save } = ctx;
    const [autostart, setAutostart] = useState(false);
    const [autostartError, setAutostartError] = useState(false);
    const [paused, setPaused] = useState(false);
    const [server, setServer] = useState<api.ServerStatus | null>(null);

    useEffect(() => {
        api.isAutostartEnabled()
            .then(setAutostart)
            .catch(() => {});
        api.isReceivingPaused()
            .then(setPaused)
            .catch(() => {});
        api.getServerStatus()
            .then(setServer)
            .catch(() => {});
    }, []);

    const refreshServer = useCallback(() => {
        api.getServerStatus()
            .then(setServer)
            .catch(() => {});
    }, []);

    // The tray menu holds the same switch, so its change has to come back here.
    useEffect(() => {
        let unlisten: (() => void) | undefined;
        let cancelled = false;
        (async () => {
            const stop = await listen<boolean>("receiving-pause-changed", (event) =>
                setPaused(event.payload),
            );
            if (cancelled) stop();
            else unlisten = stop;
        })();
        return () => {
            cancelled = true;
            unlisten?.();
        };
    }, []);

    const changeAutostart = (enabled: boolean) => {
        setAutostartError(false);
        api.setAutostart(enabled)
            .then(() => setAutostart(enabled))
            // macOS can refuse: the login item is switched off in System
            // Settings, where only the user can switch it back on.
            .catch(() => {
                setAutostartError(true);
                api.isAutostartEnabled()
                    .then(setAutostart)
                    .catch(() => {});
            });
    };

    const changePaused = (value: boolean) => {
        setPaused(value);
        api.setReceivingPaused(value).catch(() => {});
    };

    const toggleServer = (enabled: boolean) => {
        api.toggleServer(enabled)
            .catch(() => {})
            .finally(refreshServer);
    };

    return (
        <>
            <h2 className="lan-page-title">{t("general_title")}</h2>
            <LanCard>
                <LanRow label={t("autostart")} hint={t("autostart_hint")}>
                    <LanSwitch
                        checked={autostart}
                        onChange={changeAutostart}
                        label={t("autostart")}
                    />
                </LanRow>
                {autostartError ? (
                    <p className="lan-row-warning">{t("autostart_failed")}</p>
                ) : null}
                <LanRow label={t("language")}>
                    <LanSegment<LanLang>
                        value={lang}
                        ariaLabel={t("language")}
                        options={LAN_LANGS.map((item) => ({ value: item.id, label: item.label }))}
                        onChange={(next) => save("app.language", next)}
                    />
                </LanRow>
                <LanRow label={t("pause_receiving")} hint={t("pause_receiving_hint")}>
                    <LanSwitch checked={paused} onChange={changePaused} label={t("pause_receiving")} />
                </LanRow>
            </LanCard>

            <LanCard title={t("server_row")}>
                <LanRow
                    label={server?.enabled ? t("server_running", { port: server.port }) : t("server_stopped")}
                    hint={t("server_stop_hint")}
                >
                    {server?.enabled ? (
                        <LanButton onClick={() => toggleServer(false)} variant="danger">
                            {t("server_stop")}
                        </LanButton>
                    ) : (
                        <LanButton onClick={() => toggleServer(true)} variant="primary">
                            {t("server_start")}
                        </LanButton>
                    )}
                </LanRow>
            </LanCard>

            <LanCard tone="note" title={t("background_title")}>
                <p className="lan-note-body">{t("background_body")}</p>
                {/* 「启动后显示界面」 is not a switch here: this product never shows a
                    window on launch, and a control that cannot do otherwise would
                    be a lie about what the app is. */}
            </LanCard>
        </>
    );
}
