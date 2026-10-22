import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { LabelWithHint } from "../SettingsHint";

interface E2eKeyPanelProps {
    t: (key: string) => string;
    e2eKey: string;
    saveE2eKey: (value: string) => Promise<void>;
    /** Distinguishes the hint state of the two pages that show this panel. */
    hintPrefix: string;
}

/**
 * The shared key that seals MQTT messages and WebDAV files
 * (docs/E2E-ENCRYPTION.md). Shown inside both sync pages rather than on a page
 * of its own: people set up "sync", not "encryption". Both copies edit the
 * same setting.
 */
const E2eKeyPanel = ({ t, e2eKey, saveE2eKey, hintPrefix }: E2eKeyPanelProps) => {
    const [fingerprint, setFingerprint] = useState<string | null>(null);
    const [draft, setDraft] = useState("");
    const [draftInvalid, setDraftInvalid] = useState(false);
    const [revealed, setRevealed] = useState(false);
    // Rotating or removing the key cuts this device off until every other
    // device changes too, so both take a second click.
    const [armed, setArmed] = useState<"rotate" | "disable" | null>(null);

    useEffect(() => {
        if (!e2eKey) {
            setFingerprint(null);
            return;
        }
        invoke<string | null>("e2e_key_fingerprint", { key: e2eKey }).then(setFingerprint).catch(() => setFingerprint(null));
    }, [e2eKey]);

    const apply = async (value: string) => {
        await saveE2eKey(value);
        setArmed(null);
        setRevealed(false);
        setDraft("");
        setDraftInvalid(false);
    };

    const saveDraft = async () => {
        const valid = await invoke<string | null>("e2e_key_fingerprint", { key: draft }).catch(() => null);
        if (!valid) {
            setDraftInvalid(true);
            return;
        }
        await apply(draft.trim());
    };

    const confirmOr = (action: "rotate" | "disable", run: () => Promise<void>) => {
        if (armed === action) void run();
        else setArmed(action);
    };

    return (
        <>
            <h4 className="settings-section-title">{t("e2e_section")}</h4>
            <p className="settings-section-desc">{t("e2e_shared_desc")}</p>
            {fingerprint ? (
                <>
                    <div className="setting-item">
                        <LabelWithHint label={t("e2e_fingerprint")} hint={t("e2e_fingerprint_hint")} hintKey={`${hintPrefix}_e2e_fingerprint`} />
                        <code className="e2e-fingerprint">{fingerprint}</code>
                    </div>
                    <div className="setting-item">
                        <LabelWithHint label={t("e2e_key")} hint={t("e2e_share_hint")} hintKey={`${hintPrefix}_e2e_key`} />
                        <button type="button" className="settings-action-button settings-action-button--small" onClick={() => setRevealed((value) => !value)}>
                            {t(revealed ? "e2e_hide_key" : "e2e_show_key")}
                        </button>
                    </div>
                    {revealed && <code className="e2e-key-text">{e2eKey}</code>}
                    <div className="setting-item">
                        <LabelWithHint label={t("e2e_manage")} hint={t("e2e_change_warning")} hintKey={`${hintPrefix}_e2e_manage`} />
                        <div className="e2e-actions">
                            <button type="button" className="settings-action-button settings-action-button--small" onClick={() => confirmOr("rotate", async () => apply(await invoke<string>("generate_e2e_key")))}>
                                {t(armed === "rotate" ? "e2e_confirm_again" : "e2e_rotate")}
                            </button>
                            <button type="button" className="settings-action-button settings-action-button--small" onClick={() => confirmOr("disable", () => apply(""))}>
                                {t(armed === "disable" ? "e2e_confirm_again" : "e2e_disable")}
                            </button>
                        </div>
                    </div>
                </>
            ) : (
                <>
                    <div className="setting-item">
                        <LabelWithHint
                            label={t("e2e_off")}
                            hint={`${t("e2e_before_enable")}\n${t("e2e_upgrade_first")}\n${t("e2e_plaintext_dropped")}`}
                            hintKey={`${hintPrefix}_e2e_off`}
                        />
                        <button type="button" className="settings-action-button settings-action-button--small" onClick={async () => apply(await invoke<string>("generate_e2e_key"))}>
                            {t("e2e_generate")}
                        </button>
                    </div>
                    <div className="setting-item">
                        <input
                            className="search-input settings-field"
                            type="password"
                            autoComplete="off"
                            spellCheck={false}
                            value={draft}
                            placeholder={t("e2e_paste_placeholder")}
                            aria-label={t("e2e_paste_placeholder")}
                            aria-invalid={draftInvalid}
                            onChange={(e) => {
                                setDraft(e.target.value);
                                setDraftInvalid(false);
                            }}
                        />
                        <button type="button" className="settings-action-button settings-action-button--small" disabled={!draft.trim()} onClick={() => void saveDraft()}>
                            {t("e2e_save")}
                        </button>
                    </div>
                    {draftInvalid && <div className="settings-notice">{t("e2e_invalid")}</div>}
                </>
            )}
        </>
    );
};

export default E2eKeyPanel;
