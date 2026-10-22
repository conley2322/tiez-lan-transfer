import { useState, useRef, useEffect } from "react";
import {
    FolderOpen,
    Trash2,
    Search,
    MoreHorizontal,
    Globe,
    Check,
    QrCode,
} from "lucide-react";
import { QRCodeCanvas } from "qrcode.react";
import type { NetworkInterfaceInfo } from "../types";

export interface TransferHeaderProps {
    t: (key: string) => string;
    hasOnlineDevices: boolean;
    conversationTitle: string;
    onlineDevicesCount: number;
    activeIp?: string;
    actualPort?: string;
    selectedIp: string;
    interfaces: NetworkInterfaceInfo[];
    onSwitchIp: (ip: string) => void;
    showSearchBar: boolean;
    isFiltering: boolean;
    onToggleSearch: () => void;
    onOpenFolder: () => void;
    hasMessages: boolean;
    onClearHistory: () => void;
    connectionUrl?: string;
    copiedLink?: boolean;
    onCopyLink?: (url: string) => void;
    showQrCode?: boolean;
    onToggleQrCode?: () => void;
}

export const TransferHeader = ({
    t,
    hasOnlineDevices,
    conversationTitle,
    onlineDevicesCount,
    selectedIp,
    interfaces,
    onSwitchIp,
    showSearchBar,
    isFiltering,
    onToggleSearch,
    onOpenFolder,
    hasMessages,
    onClearHistory,
    activeIp,
    actualPort,
    connectionUrl,
    copiedLink,
    onCopyLink,
    showQrCode,
    onToggleQrCode,
}: TransferHeaderProps) => {
    const [showMoreMenu, setShowMoreMenu] = useState(false);
    const moreMenuRef = useRef<HTMLDivElement>(null);

    // Close the menu and the pairing code when clicking outside
    useEffect(() => {
        if (!showMoreMenu && !showQrCode) return;
        const handleClickOutside = (e: MouseEvent) => {
            if (!moreMenuRef.current?.contains(e.target as Node)) {
                setShowMoreMenu(false);
                if (showQrCode) onToggleQrCode?.();
            }
        };
        window.addEventListener("pointerdown", handleClickOutside);
        return () => window.removeEventListener("pointerdown", handleClickOutside);
    }, [showMoreMenu, showQrCode, onToggleQrCode]);

    return (
        <div className="wt-header">
            {/* Left: Clear connection status indicator */}
            <div className="wt-header-status-group">
                <span
                    className={`wt-status-dot ${hasOnlineDevices ? "is-online" : "is-waiting"}`}
                    aria-hidden="true"
                />
                <div className="wt-header-status-copy">
                    <span className="wt-status-primary">
                        {hasOnlineDevices
                            ? (conversationTitle || t("ft_device_ready"))
                            : t("ft_waiting_device")}
                    </span>
                    {hasOnlineDevices && (
                        <span className="wt-status-secondary">
                            {onlineDevicesCount === 1 ? t("ft_lan_transferring") : t("ft_devices_online").replace("{n}", String(onlineDevicesCount))}
                        </span>
                    )}
                </div>
            </div>

            {/* Right: Only primary Search button + "三个点" (...) More Menu button */}
            <div className="wt-header-actions" ref={moreMenuRef}>
                {/* The pairing code stays reachable once a device is connected,
                    for another phone or a reconnect. */}
                {connectionUrl && (
                    <button
                        type="button"
                        className={`wt-header-action ${showQrCode ? "is-active" : ""}`}
                        title={t("ft_pair_title")}
                        aria-label={t("ft_pair_title")}
                        aria-expanded={showQrCode}
                        onClick={() => {
                            setShowMoreMenu(false);
                            onToggleQrCode?.();
                        }}
                    >
                        <QrCode size={15} strokeWidth={2.2} />
                    </button>
                )}

                <button
                    type="button"
                    className={`wt-header-action ${showSearchBar || isFiltering ? "is-active" : ""}`}
                    title={showSearchBar ? t("ft_close_search") : t("ft_search_records_hotkey")}
                    aria-label={t("ft_search_records")}
                    onClick={onToggleSearch}
                >
                    <Search size={15} strokeWidth={2.2} />
                </button>

                <button
                    type="button"
                    className={`wt-header-action ${showMoreMenu ? "is-active" : ""}`}
                    title={t("more_actions")}
                    aria-label={t("more_actions")}
                    aria-expanded={showMoreMenu}
                    onClick={() => {
                        if (showQrCode) onToggleQrCode?.();
                        setShowMoreMenu((prev) => !prev);
                    }}
                >
                    <MoreHorizontal size={18} strokeWidth={2.4} />
                </button>

                {showQrCode && connectionUrl && (
                    <div className="wt-qr-popover" role="dialog" aria-label={t("ft_pair_title")}>
                        <div className="wt-pairing-hero">
                            <div className="wt-pairing-qr">
                                <QRCodeCanvas value={connectionUrl} size={144} />
                            </div>
                            <div className="wt-pairing-title">{t("ft_pair_title")}</div>
                            <div className="wt-pairing-tip">{t("ft_pair_tip")}</div>
                            <div className="wt-pairing-address-row">
                                <span className="wt-pairing-address">{activeIp}:{actualPort}</span>
                                <button
                                    type="button"
                                    className="wt-pairing-copy-btn"
                                    onClick={() => onCopyLink?.(connectionUrl)}
                                    title={t("ft_copy_connect_url")}
                                >
                                    {copiedLink ? t("ft_copied") : t("ft_copy_address")}
                                </button>
                            </div>
                        </div>
                    </div>
                )}

                {/* "三个点" 更多操作下拉菜单 */}
                {showMoreMenu && (
                    <div className="wt-more-menu" role="menu">
                        <button
                            type="button"
                            className="wt-more-menu-item"
                            role="menuitem"
                            onClick={() => {
                                setShowMoreMenu(false);
                                onOpenFolder();
                            }}
                        >
                            <FolderOpen size={16} strokeWidth={1.8} />
                            <span>{t("ft_open_save_folder")}</span>
                        </button>

                        {interfaces.length > 1 && (
                            <>
                                <div className="wt-more-menu-divider" role="separator" />
                                <div className="wt-more-menu-section-label">{t("ft_switch_network")}</div>
                                {interfaces.map((iface) => (
                                    <button
                                        key={iface.ip}
                                        type="button"
                                        className={`wt-more-menu-item ${selectedIp === iface.ip ? "is-selected" : ""}`}
                                        role="menuitemradio"
                                        aria-checked={selectedIp === iface.ip}
                                        onClick={() => {
                                            onSwitchIp(iface.ip);
                                        }}
                                    >
                                        <Globe size={16} strokeWidth={1.8} />
                                        <span className="wt-more-menu-ip-label">{iface.label}</span>
                                        {selectedIp === iface.ip && <Check size={15} strokeWidth={2.4} className="wt-more-menu-check" />}
                                    </button>
                                ))}
                            </>
                        )}

                        {hasMessages && (
                            <>
                                <div className="wt-more-menu-divider" role="separator" />
                                <button
                                    type="button"
                                    className="wt-more-menu-item is-danger"
                                    role="menuitem"
                                    onClick={() => {
                                        setShowMoreMenu(false);
                                        onClearHistory();
                                    }}
                                >
                                    <Trash2 size={16} strokeWidth={1.8} />
                                    <span>{t("ft_clear_history")}</span>
                                </button>
                            </>
                        )}
                    </div>
                )}
            </div>
        </div>
    );
};
