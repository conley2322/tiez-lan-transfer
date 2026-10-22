// 右键菜单 — one page per platform, and both halves say what is true.
//
// On Windows the switch is the send entry point itself: Rust writes the submenu
// into HKCU or deletes it, and this page prints the names read back out of the
// registry, so 「开」 can only ever mean 「菜单里真的有这些项」. On macOS the entry
// would need a FinderSync extension signed together with the app, which this
// build does not have, so the page says so and points at the drop box.

import { useCallback, useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import * as api from "../api";
import { LanButton, LanCard, LanPill, LanRow, LanSwitch } from "../controls";
import type { SettingsCtx } from "../LanSettingsWindow";

export default function MenuPage({ ctx, isMac }: { ctx: SettingsCtx; isMac: boolean }) {
    const { t, go } = ctx;
    const [menu, setMenu] = useState<api.WinMenuState | null>(null);
    const [error, setError] = useState("");

    const refresh = useCallback(() => {
        if (isMac) return;
        api.getWinMenuState()
            .then(setMenu)
            .catch(() => {});
    }, [isMac]);

    useEffect(refresh, [refresh]);

    // Pairing, renaming or removing a machine rewrites the submenu in Rust, so
    // this list follows that event instead of trusting what the page last read.
    useEffect(() => {
        if (isMac) return;
        let unlisten: (() => void) | undefined;
        let cancelled = false;
        (async () => {
            const stop = await listen("peers-changed", refresh);
            if (cancelled) stop();
            else unlisten = stop;
        })();
        return () => {
            cancelled = true;
            unlisten?.();
        };
    }, [refresh, isMac]);

    const toggle = (enabled: boolean) => {
        setError("");
        api.setWinMenuEnabled(enabled)
            .then(refresh)
            .catch((reason) => setError(t("menu_win_failed", { reason: String(reason) })));
    };

    const entries = menu?.entries ?? [];

    return (
        <>
            <h2 className="lan-page-title">{t("menu_title")}</h2>

            <LanCard title={t("menu_mac_head")}>
                <p className="lan-note-body">{t("menu_mac_body")}</p>
                <p className="lan-note-body">{t("menu_mac_use")}</p>
                {isMac ? <LanButton onClick={() => go("corner")}>{t("menu_goto_corner")}</LanButton> : null}
            </LanCard>

            <LanCard title={t("menu_win_head")}>
                <p className="lan-note-body">{t("menu_win_body")}</p>

                {isMac ? (
                    <p className="lan-row-warning">{t("menu_win_on_mac")}</p>
                ) : (
                    <>
                        <LanRow label={t("menu_win_toggle")} hint={t("menu_win_toggle_hint")}>
                            <LanSwitch
                                checked={menu?.enabled ?? true}
                                onChange={toggle}
                                label={t("menu_win_toggle")}
                            />
                        </LanRow>

                        <div className="lan-menu-entries">
                            <span className="lan-menu-entries-title">{t("menu_win_entries")}</span>
                            {entries.length > 0 ? (
                                // Two machines can carry the same nickname, and the
                                // list is the registry read back verbatim — so the
                                // name alone is not a key.
                                entries.map((name, index) => (
                                    <LanPill key={`${name}-${index}`}>{name}</LanPill>
                                ))
                            ) : (
                                <span className="lan-menu-entries-empty">
                                    {t("menu_win_entries_empty")}
                                </span>
                            )}
                        </div>

                        {menu && menu.exePath ? (
                            <p className="lan-note-body">{t("menu_win_exe", { path: menu.exePath })}</p>
                        ) : null}
                        <p className="lan-note-body">{t("menu_win_more")}</p>
                        <p className="lan-row-warning">{t("menu_win_admin")}</p>
                        {error ? <p className="lan-inline-result">{error}</p> : null}
                    </>
                )}
            </LanCard>
        </>
    );
}
