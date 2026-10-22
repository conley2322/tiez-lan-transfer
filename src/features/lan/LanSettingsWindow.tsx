// 设置 — the only window this app shows on request (window=settings).
//
// Reached from the menu-bar/tray icon's right-click menu, so it is a full
// settings app in one page rather than a panel inside the clipboard window. The
// tab arrives two ways: as `?tab=` on the very first load, and as a
// `settings-tab` event afterwards, because a window Tauri keeps alive cannot be
// re-navigated by its query string (see app/floating_windows.rs).

import { useCallback, useEffect, useMemo, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import {
    ArrowLeftRight,
    Frame,
    History,
    Info,
    Menu as MenuIcon,
    SlidersHorizontal,
    Users,
} from "lucide-react";
import * as api from "./api";
import type { CornerState, FloatState, Peer, SelfIdentity } from "./api";
import { lanT, readLanLang, type LanKey, type LanLang, type LanT } from "./lanCopy";
import GeneralPage from "./pages/GeneralPage";
import DevicesPage from "./pages/DevicesPage";
import TransferPage from "./pages/TransferPage";
import CornerPage from "./pages/CornerPage";
import MenuPage from "./pages/MenuPage";
import HistoryPage from "./pages/HistoryPage";
import AboutPage from "./pages/AboutPage";

export type LanTab = "general" | "devices" | "transfer" | "corner" | "menu" | "history" | "about";

const TABS: Array<{ id: LanTab; key: LanKey; icon: typeof Users }> = [
    { id: "general", key: "tab_general", icon: SlidersHorizontal },
    { id: "devices", key: "tab_devices", icon: Users },
    { id: "transfer", key: "tab_transfer", icon: ArrowLeftRight },
    { id: "corner", key: "tab_corner", icon: Frame },
    { id: "menu", key: "tab_menu", icon: MenuIcon },
    { id: "history", key: "tab_history", icon: History },
    { id: "about", key: "tab_about", icon: Info },
];

/** The tray menu's 传输记录… opens this tab; older builds called it `transfers`. */
const TAB_ALIASES: Record<string, LanTab> = {
    transfers: "history",
    peers: "devices",
    general: "general",
    devices: "devices",
    transfer: "transfer",
    corner: "corner",
    menu: "menu",
    history: "history",
    about: "about",
};

const readInitialTab = (): LanTab => {
    const query = new URLSearchParams(window.location.search).get("tab") ?? "";
    return TAB_ALIASES[query] ?? "general";
};

export type SettingsCtx = {
    t: LanT;
    lang: LanLang;
    settings: Record<string, string>;
    save: (key: string, value: string) => void;
    peers: Peer[];
    refreshPeers: () => void;
    identity: SelfIdentity | null;
    refreshIdentity: () => void;
    corner: CornerState | null;
    refreshCorner: () => void;
    float: FloatState | null;
    refreshFloat: () => void;
    go: (tab: LanTab) => void;
};

const isMacPlatform = /Mac|iPhone|iPad|iPod/i.test(navigator.userAgent);

export default function LanSettingsWindow() {
    const [tab, setTab] = useState<LanTab>(readInitialTab);
    const [settings, setSettings] = useState<Record<string, string>>({});
    const [lang, setLang] = useState<LanLang>("zh");
    const [peers, setPeers] = useState<Peer[]>([]);
    const [identity, setIdentity] = useState<SelfIdentity | null>(null);
    const [corner, setCorner] = useState<CornerState | null>(null);
    const [float, setFloat] = useState<FloatState | null>(null);

    const t = useMemo(() => lanT(lang), [lang]);

    useEffect(() => {
        api.getSettings()
            .then((loaded) => {
                setSettings(loaded);
                setLang(readLanLang(loaded));
            })
            .catch(() => {});
    }, []);

    const refreshPeers = useCallback(() => {
        api.listPeers()
            .then(setPeers)
            .catch(() => {});
    }, []);

    const refreshIdentity = useCallback(() => {
        api.getIdentity()
            .then(setIdentity)
            .catch(() => {});
    }, []);

    const refreshCorner = useCallback(() => {
        api.getCornerState()
            .then(setCorner)
            .catch(() => {});
    }, []);

    const refreshFloat = useCallback(() => {
        api.getFloatState()
            .then(setFloat)
            .catch(() => {});
    }, []);

    useEffect(() => {
        refreshPeers();
        refreshIdentity();
        refreshCorner();
        refreshFloat();
    }, [refreshPeers, refreshIdentity, refreshCorner, refreshFloat]);

    // A peer only ages into 离线 when its beacon stops arriving, so the list
    // needs its own clock rather than waiting for an event that never comes.
    useEffect(() => {
        let unlisten: (() => void) | undefined;
        let cancelled = false;
        (async () => {
            const stop = await listen("peers-changed", refreshPeers);
            if (cancelled) stop();
            else unlisten = stop;
        })();
        const timer = window.setInterval(refreshPeers, 5000);
        return () => {
            cancelled = true;
            window.clearInterval(timer);
            unlisten?.();
        };
    }, [refreshPeers]);

    useEffect(() => {
        let unlisten: (() => void) | undefined;
        let cancelled = false;
        (async () => {
            const stop = await listen<string>("settings-tab", (event) => {
                setTab(TAB_ALIASES[event.payload] ?? "general");
            });
            if (cancelled) stop();
            else unlisten = stop;
        })();
        return () => {
            cancelled = true;
            unlisten?.();
        };
    }, []);

    /** Write, then do whatever the value is *for*. A setting that only lands in
     *  the database would be a switch that changes nothing. */
    const save = useCallback((key: string, value: string) => {
        setSettings((current) => ({ ...current, [key]: value }));
        if (key === "app.language") {
            const next = readLanLang({ [key]: value });
            setLang(next);
            localStorage.setItem("tiez_language", next);
        }
        api.saveSetting(key, value).catch(() => {});
        if (key.startsWith("peer_progress_")) {
            api.syncFloatLayout().catch(() => {});
            api.getFloatState()
                .then(setFloat)
                .catch(() => {});
        }
    }, []);

    const ctx: SettingsCtx = {
        t,
        lang,
        settings,
        save,
        peers,
        refreshPeers,
        identity,
        refreshIdentity,
        corner,
        refreshCorner,
        float,
        refreshFloat,
        go: setTab,
    };

    return (
        <div className="lan-settings">
            <nav className="lan-side" aria-label={t("nav_group")}>
                <div className="lan-side-group">{t("nav_group")}</div>
                {TABS.map((item) => {
                    const Icon = item.icon;
                    const active = item.id === tab;
                    return (
                        <button
                            key={item.id}
                            type="button"
                            className={active ? "lan-side-item is-on" : "lan-side-item"}
                            aria-current={active ? "page" : undefined}
                            onClick={() => setTab(item.id)}
                        >
                            <Icon size={16} />
                            {t(item.key)}
                        </button>
                    );
                })}
            </nav>
            <main className="lan-content">
                {tab === "general" ? <GeneralPage ctx={ctx} /> : null}
                {tab === "devices" ? <DevicesPage ctx={ctx} /> : null}
                {tab === "transfer" ? <TransferPage ctx={ctx} /> : null}
                {tab === "corner" ? <CornerPage ctx={ctx} isMac={isMacPlatform} /> : null}
                {tab === "menu" ? <MenuPage ctx={ctx} isMac={isMacPlatform} /> : null}
                {tab === "history" ? <HistoryPage ctx={ctx} /> : null}
                {tab === "about" ? <AboutPage ctx={ctx} isMac={isMacPlatform} /> : null}
            </main>
        </div>
    );
}
