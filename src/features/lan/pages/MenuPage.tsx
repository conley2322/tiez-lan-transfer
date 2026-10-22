// 右键菜单 — a status page, not a switchboard.
//
// Neither sender exists yet in this build: the Mac side needs a FinderSync
// system extension signed together with the app (there is no such target in this
// repository), and the Windows shell submenu was deliberately left for the
// Windows build. Putting toggles here would mean controls that change nothing,
// so the page says what is true and points at the one way that works today.

import { LanButton, LanCard } from "../controls";
import type { SettingsCtx } from "../LanSettingsWindow";

export default function MenuPage({ ctx, isMac }: { ctx: SettingsCtx; isMac: boolean }) {
    const { t, go } = ctx;

    return (
        <>
            <h2 className="lan-page-title">{t("menu_title")}</h2>

            <LanCard title={t("menu_mac_head")}>
                <p className="lan-note-body">{t("menu_mac_body")}</p>
                <p className="lan-note-body">{t("menu_mac_use")}</p>
                <LanButton onClick={() => go("corner")}>{t("menu_goto_corner")}</LanButton>
            </LanCard>

            <LanCard title={t("menu_win_head")}>
                <p className="lan-note-body">{t("menu_win_body")}</p>
            </LanCard>

            {isMac ? null : (
                <LanCard tone="note">
                    <p className="lan-note-body">{t("corner_mac_only")}</p>
                </LanCard>
            )}
        </>
    );
}
