import type { ReactNode } from "react";
import { Check, Copy, ExternalLink, Github, Globe, MessageSquare, RotateCcw } from "lucide-react";
import { openUrl } from "@tauri-apps/plugin-opener";
import appLogo from "../../../../src-tauri/icons/128x128@2x.png";
import type { ReleaseChannel } from "../../../shared/lib/releaseBuild";
import BetaUpdatesSetting from "./BetaUpdatesSetting";

interface SettingsAboutProps {
    t: (key: string) => string;
    appVersion: string;
    releaseChannel: ReleaseChannel;
    portable: boolean;
    updateButton: ReactNode;
    emailCopied: boolean;
    onCopyEmail: () => void;
    onResetSettings: () => void;
}

const SettingsAbout = ({ t, appVersion, releaseChannel, portable, updateButton, emailCopied, onCopyEmail, onResetSettings }: SettingsAboutProps) => (
    <section className="settings-about" aria-label={t("settings_about")}>
        <header className="settings-about-header">
            <div className="settings-about-identity">
                <img className="settings-about-icon" src={appLogo} alt="TieZ Logo" />
                <div>
                    <h2>TieZ</h2>
                    <p className="settings-about-slogan">{t("slogan")}</p>
                </div>
            </div>
            <p className="settings-about-description">{t("about_description")}</p>
        </header>

        <div className="settings-about-version">
            <div className="settings-about-version-info">
                <span className="settings-about-caption">{t("about_version")}</span>
                <div className="settings-about-version-number">
                    <strong>{appVersion ? `v${appVersion}` : t("loading")}</strong>
                    {releaseChannel === "beta" && <span className="settings-version-badge beta">Beta</span>}
                    {portable && <span className="settings-version-badge portable">Portable</span>}
                </div>
            </div>
            {updateButton}
        </div>

        <BetaUpdatesSetting t={t} releaseChannel={releaseChannel} portable={portable} />

        <section className="settings-about-resources" aria-labelledby="settings-about-links-title">
            <h3 id="settings-about-links-title">{t("about_links")}</h3>
            <div className="settings-about-links">
                <button type="button" className="settings-about-link" onClick={() => void openUrl("https://tiez.name666.top/")}>
                    <Globe size={18} aria-hidden="true" />
                    <span className="settings-about-link-text">
                        <strong>{t("official_website")}</strong>
                        <span>{t("about_website_desc")}</span>
                    </span>
                    <ExternalLink size={14} className="settings-about-link-end" aria-hidden="true" />
                </button>
                <button type="button" className="settings-about-link" onClick={() => void openUrl("https://github.com/jimuzhe/tiez-clipboard")}>
                    <Github size={18} aria-hidden="true" />
                    <span className="settings-about-link-text">
                        <strong>GitHub</strong>
                        <span>{t("about_github_desc")}</span>
                    </span>
                    <ExternalLink size={14} className="settings-about-link-end" aria-hidden="true" />
                </button>
                <button type="button" className="settings-about-link" onClick={onCopyEmail} title={t("about_feedback_desc")}>
                    <MessageSquare size={18} aria-hidden="true" />
                    <span className="settings-about-link-text">
                        <strong aria-live="polite">{emailCopied ? t("email_copied") : t("feedback")}</strong>
                        <span>tiez@name666.top</span>
                    </span>
                    {emailCopied ? <Check size={14} className="settings-about-link-end" aria-hidden="true" /> : <Copy size={14} className="settings-about-link-end" aria-hidden="true" />}
                </button>
            </div>
        </section>

        <div className="settings-about-reset">
            <div>
                <h3>{t("reset_defaults")}</h3>
                <p>{t("about_reset_desc")}</p>
            </div>
            <button type="button" className="settings-action-button" onClick={onResetSettings}>
                <RotateCcw size={14} aria-hidden="true" />
                <span>{t("reset_defaults")}</span>
            </button>
        </div>
    </section>
);

export default SettingsAbout;
