import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import {
    BETA_AVAILABLE_EVENT,
    RECEIVE_BETA_SETTING,
    getAvailableBetaVersion,
    requestBetaUpdateDialog
} from "../../../shared/hooks/useAutoUpdate";
import type { ReleaseChannel } from "../../../shared/lib/releaseBuild";

interface BetaUpdatesSettingProps {
    t: (key: string) => string;
    releaseChannel: ReleaseChannel;
    portable: boolean;
}

/**
 * Stable builds only: opt into beta update prompts. Without opting in, a
 * newer beta is shown here as a line, never as a dialog. Beta builds already
 * get both betas and stable releases.
 */
const BetaUpdatesSetting = ({ t, releaseChannel, portable }: BetaUpdatesSettingProps) => {
    const [enabled, setEnabled] = useState(false);
    const [betaVersion, setBetaVersion] = useState<string | null>(getAvailableBetaVersion);
    const shown = releaseChannel === "stable" && !portable;

    useEffect(() => {
        if (!shown) return;
        invoke<Record<string, string>>("get_settings")
            .then((settings) => setEnabled(settings[RECEIVE_BETA_SETTING] === "true"))
            .catch(console.error);
        let unlisten: (() => void) | undefined;
        listen<string | null>(BETA_AVAILABLE_EVENT, (event) => setBetaVersion(event.payload))
            .then((fn) => { unlisten = fn; });
        return () => unlisten?.();
    }, [shown]);

    if (!shown) return null;

    return (
        <div className="setting-item beta-updates-setting">
            <div className="item-label-group">
                <span className="item-label">{t("receive_beta_updates")}</span>
                <span className="hint">{t("receive_beta_updates_hint")}</span>
                {betaVersion && (
                    <button type="button" className="beta-updates-available" onClick={() => void requestBetaUpdateDialog()}>
                        <span className="settings-version-badge beta">{t("beta_label")}</span>
                        <span>{t("beta_update_available").replace("{version}", betaVersion)}</span>
                    </button>
                )}
            </div>
            <label className="switch">
                <input
                    className="cb"
                    type="checkbox"
                    checked={enabled}
                    aria-label={t("receive_beta_updates")}
                    onChange={(e) => {
                        const value = e.target.checked;
                        setEnabled(value);
                        invoke("save_setting", { key: RECEIVE_BETA_SETTING, value: String(value) }).catch(console.error);
                    }}
                />
                <div className="toggle"><div className="left" /><div className="right" /></div>
            </label>
        </div>
    );
};

export default BetaUpdatesSetting;
