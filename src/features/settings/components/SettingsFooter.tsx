import { Github, MessageSquare, RotateCcw } from "lucide-react";
import { openUrl } from "@tauri-apps/plugin-opener";
import { emit, listen } from "@tauri-apps/api/event";
import {
    CHECK_RESULT_EVENT,
    type UpdateCheckResult
} from "../../../shared/hooks/useAutoUpdate";
import SettingsAbout from "./SettingsAbout";
import BetaUpdatesSetting from "./BetaUpdatesSetting";
import type { ReleaseChannel } from "../../../shared/lib/releaseBuild";

interface SettingsFooterProps {
    expanded?: boolean;
    t: (key: string) => string;
    appVersion: string;
    releaseChannel: ReleaseChannel;
    portable: boolean;
    updateStatus: string;
    setUpdateStatus: (val: string) => void;
    onResetSettings: () => void;
    emailCopied: boolean;
    setEmailCopied: (val: boolean) => void;
}

const SettingsFooter = ({
    expanded = false,
    t,
    appVersion,
    releaseChannel,
    portable,
    updateStatus,
    setUpdateStatus,
    onResetSettings,
    emailCopied,
    setEmailCopied
}: SettingsFooterProps) => {
    const copyEmail = () => {
        navigator.clipboard.writeText('tiez@name666.top');
        setEmailCopied(true);
        setTimeout(() => setEmailCopied(false), 2000);
    };
    const updateButton = (
        <button
            type="button"
            className={expanded ? `settings-action-button settings-about-update${updateStatus === t('checking_failed') ? ' is-error' : updateStatus === t('up_to_date') ? ' is-success' : ''}` : undefined}
            onClick={async () => {
                if (updateStatus) return;
                setUpdateStatus(t('checking'));

                const requestId = `${Date.now()}-${Math.random().toString(36).slice(2)}`;
                let settled = false;
                let timeoutId: number | undefined;
                const unlisten = await listen<UpdateCheckResult>(CHECK_RESULT_EVENT, (event) => {
                    if (event.payload.requestId !== requestId || settled) return;
                    settled = true;
                    if (timeoutId !== undefined) window.clearTimeout(timeoutId);
                    unlisten();

                    const labels: Record<UpdateCheckResult["status"], string> = {
                        available: t('new_version_found'),
                        "up-to-date": t('up_to_date'),
                        error: t('checking_failed'),
                        portable: t('portable_manual_update')
                    };
                    setUpdateStatus(labels[event.payload.status]);
                    window.setTimeout(() => setUpdateStatus(''), event.payload.status === "portable" ? 4000 : 2000);
                });

                timeoutId = window.setTimeout(() => {
                    if (settled) return;
                    settled = true;
                    unlisten();
                    setUpdateStatus(t('checking_failed'));
                    window.setTimeout(() => setUpdateStatus(''), 2000);
                }, 12000);

                await emit("check-update-manually", { requestId });
            }}
            disabled={!!updateStatus}
            style={expanded ? undefined : {
                border: 'none',
                background: 'transparent',
                color: (updateStatus && (updateStatus === t('checking_failed'))) ? 'var(--status-danger)' :
                       (updateStatus === t('up_to_date')) ? 'var(--status-success)' : 'var(--accent-color)',
                cursor: updateStatus ? 'default' : 'pointer',
                fontSize: 'var(--font-size-xs)',
                padding: '2px 6px',
                borderRadius: '4px',
                opacity: updateStatus ? 1 : 0.8,
                fontWeight: updateStatus ? 'bold' : 'normal',
                transition: 'all 0.2s'
            }}
            onMouseEnter={(e) => !expanded && !updateStatus && (e.currentTarget.style.opacity = '1')}
            onMouseLeave={(e) => !expanded && !updateStatus && (e.currentTarget.style.opacity = '0.8')}
        >
            {updateStatus || t('check_update')}
        </button>
    );

    if (expanded) {
        return (
            <SettingsAbout
                t={t}
                appVersion={appVersion}
                releaseChannel={releaseChannel}
                portable={portable}
                updateButton={updateButton}
                emailCopied={emailCopied}
                onCopyEmail={copyEmail}
                onResetSettings={onResetSettings}
            />
        );
    }

    return (
        <>
            {/* Footer Actions */}
            <div style={{
                marginTop: '16px',
                display: 'flex',
                justifyContent: 'center',
                gap: '12px',
                flexWrap: 'wrap'
            }}>
                {/* Feedback Card */}
                <div
                    className="settings-group"
                    style={{
                        cursor: 'pointer',
                        transition: 'all 0.2s',
                        margin: 0,
                        width: 'auto',
                        padding: '10px 16px',
                        display: 'flex',
                        alignItems: 'center',
                        justifyContent: 'center',
                        marginBottom: '0'
                    }}
                    onClick={copyEmail}
                >
                    <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
                        <MessageSquare size={16} />
                        <span style={{ fontSize: 'var(--font-size-base)', fontWeight: 600 }}>
                            {emailCopied ? t('email_copied') : t('feedback')}
                        </span>
                    </div>
                </div>

                {/* Reset Card */}
                <div
                    className="settings-group"
                    style={{
                        cursor: 'pointer',
                        transition: 'all 0.2s',
                        margin: 0,
                        width: 'auto',
                        padding: '10px 16px',
                        display: 'flex',
                        alignItems: 'center',
                        justifyContent: 'center',
                        marginBottom: '0'
                    }}
                    onClick={() => onResetSettings()}
                >
                    <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
                        <RotateCcw size={16} />
                        <span style={{ fontSize: 'var(--font-size-base)', fontWeight: 600 }}>{t('reset_defaults')}</span>
                    </div>
                </div>
            </div>

            {releaseChannel === "stable" && !portable && (
                <div className="settings-group beta-updates-group">
                    <div className="group-content">
                        <BetaUpdatesSetting t={t} releaseChannel={releaseChannel} portable={portable} />
                    </div>
                </div>
            )}

            {/* Version Info */}
            <div style={{
                marginTop: '16px',
                marginBottom: '32px',
                textAlign: 'center',
                opacity: 1
            }}>
                <div style={{
                    fontSize: 'var(--font-size-base)',
                    fontWeight: 600,
                    color: 'var(--text-secondary)',
                    letterSpacing: '0.5px',
                    marginBottom: '4px',
                    display: 'flex',
                    alignItems: 'center',
                    justifyContent: 'center',
                    gap: '8px'
                }}>
                    <span>TieZ {appVersion ? `v${appVersion}` : "v0.2.0"}</span>
                    {releaseChannel === "beta" && (
                        <span className="settings-version-badge beta">Beta</span>
                    )}
                    {portable && (
                        <span className="settings-version-badge portable">Portable</span>
                    )}
                    {updateButton}
                </div>
                <div style={{
                    fontSize: 'var(--font-size-xs)',
                    color: 'var(--text-secondary)',
                    fontWeight: 500,
                    marginBottom: '4px'
                }}>
                    {t('slogan')}
                </div>
                <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'center', gap: '10px', flexWrap: 'wrap' }}>
                    <button
                        onClick={() => openUrl('https://tiez.name666.top/')}
                        style={{
                            fontSize: 'var(--font-size-xs)',
                            color: 'var(--accent-color)',
                            background: 'transparent',
                            border: 'none',
                            cursor: 'pointer',
                            textDecoration: 'underline',
                            opacity: 0.7,
                            fontWeight: 600,
                            padding: '2px 4px'
                        }}
                        onMouseEnter={(e) => (e.currentTarget.style.opacity = '1')}
                        onMouseLeave={(e) => (e.currentTarget.style.opacity = '0.7')}
                    >
                        {t('official_website')}
                    </button>
                    <button
                        onClick={() => openUrl('https://github.com/jimuzhe/tiez-clipboard')}
                        style={{
                            fontSize: 'var(--font-size-xs)',
                            color: 'var(--accent-color)',
                            background: 'transparent',
                            border: 'none',
                            cursor: 'pointer',
                            textDecoration: 'underline',
                            opacity: 0.7,
                            fontWeight: 600,
                            padding: '2px 4px',
                            display: 'inline-flex',
                            alignItems: 'center',
                            gap: '4px'
                        }}
                        onMouseEnter={(e) => (e.currentTarget.style.opacity = '1')}
                        onMouseLeave={(e) => (e.currentTarget.style.opacity = '0.7')}
                    >
                        <Github size={12} />
                        <span>GitHub</span>
                    </button>
                </div>
            </div>
        </>
    );
};

export default SettingsFooter;
