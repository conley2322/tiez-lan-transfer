import type { ComponentType, ReactNode } from "react";
import { invoke } from "@tauri-apps/api/core";
import { ChevronDown, ChevronRight } from "lucide-react";
import { isMacPlatform, isWindowsPlatform } from "../../../../shared/lib/platform";
import MacAccessibilityPermission from "./MacAccessibilityPermission";
import { SettingRange, SettingSwitch } from "../SettingControls";

interface LabelWithHintProps {
    label: string;
    hint?: string | ReactNode;
    hintKey: string;
}

interface GeneralSettingsGroupProps {
    t: (key: string) => string;
    collapsed: boolean;
    onToggle: () => void;
    LabelWithHint: ComponentType<LabelWithHintProps>;
    autoStart: boolean;
    setAutoStart: (val: boolean) => void;
    elevatedAutostart: boolean;
    setElevatedAutostart: (val: boolean) => void;
    silentStart: boolean;
    setSilentStart: (val: boolean) => void;
    hideTrayIcon: boolean;
    setHideTrayIcon: (val: boolean) => void;
    hideDockIcon: boolean;
    setHideDockIcon: (val: boolean) => void;
    edgeDocking: boolean;
    setEdgeDocking: (val: boolean) => void;
    followMouse: boolean;
    setFollowMouse: (val: boolean) => void;
    soundEnabled: boolean;
    setSoundEnabled: (val: boolean) => void;
    pasteSoundEnabled: boolean;
    setPasteSoundEnabled: (val: boolean) => void;
    searchBoxPinned: boolean;
    setSearchBoxPinned: (val: boolean) => void;
    setShowSearchBox: (val: boolean) => void;
    scrollTopButtonEnabled: boolean;
    setScrollTopButtonEnabled: (val: boolean) => void;
    emojiPanelEnabled: boolean;
    setEmojiPanelEnabled: (val: boolean) => void;
    tagManagerEnabled: boolean;
    setTagManagerEnabled: (val: boolean) => void;
    arrowKeySelection: boolean;
    setArrowKeySelection: (val: boolean) => void;
    lightModeEnabled: boolean;
    setLightModeEnabled: (val: boolean) => void;
    lightModeIdleMinutes: number;
    setLightModeIdleMinutes: (val: number) => void;
    soundVolume: number;
    setSoundVolume: (val: number) => void;
    saveAppSetting: (key: string, val: string) => void;
}

const applyToggle = (
    next: boolean,
    setValue: (val: boolean) => void,
    command: string,
    args: Record<string, unknown>,
    onSuccess?: () => void
) => {
    setValue(next);
    invoke(command, args)
        .then(() => onSuccess?.())
        .catch((error) => {
            setValue(!next);
            console.error(`Failed to apply ${command}:`, error);
        });
};

const playPreview = (kind: "copy" | "paste") => {
    invoke("play_preview_sound", { kind }).catch(console.error);
};

const GeneralSettingsGroup = ({
    t,
    collapsed,
    onToggle,
    LabelWithHint,
    autoStart,
    setAutoStart,
    elevatedAutostart,
    setElevatedAutostart,
    silentStart,
    setSilentStart,
    hideTrayIcon,
    setHideTrayIcon,
    hideDockIcon,
    setHideDockIcon,
    edgeDocking,
    setEdgeDocking,
    followMouse,
    setFollowMouse,
    soundEnabled,
    setSoundEnabled,
    pasteSoundEnabled,
    setPasteSoundEnabled,
    searchBoxPinned,
    setSearchBoxPinned,
    setShowSearchBox,
    scrollTopButtonEnabled,
    setScrollTopButtonEnabled,
    emojiPanelEnabled,
    setEmojiPanelEnabled,
    tagManagerEnabled,
    setTagManagerEnabled,
    arrowKeySelection,
    setArrowKeySelection,
    lightModeEnabled,
    setLightModeEnabled,
    lightModeIdleMinutes,
    setLightModeIdleMinutes,
    soundVolume,
    setSoundVolume,
    saveAppSetting
}: GeneralSettingsGroupProps) => (
    <div className={`settings-group ${collapsed ? 'collapsed' : ''}`}>
        <button type="button" className="group-header" aria-expanded={!collapsed} onClick={onToggle}>
            <h3>{t('general_settings')}</h3>
            {collapsed ? <ChevronRight size={16} /> : <ChevronDown size={16} />}
        </button>
        {!collapsed && (
            <div className="group-content">
                <h4 className="settings-section-title">{t('settings_section_startup')}</h4>
                <div className="setting-item">
                    <LabelWithHint label={t('autostart')} hint={t('autostart_hint')} hintKey="autostart" />
                    <SettingSwitch
                        checked={autoStart}
                        onChange={(e) => {
                            const enabled = e.target.checked;
                            if (!enabled) {
                                setElevatedAutostart(false);
                            }
                            applyToggle(enabled, setAutoStart, "toggle_autostart", { enabled });
                        }}
                    />
                </div>

                {isWindowsPlatform() && (
                    <div className={`setting-item is-child${autoStart ? '' : ' is-disabled'}`}>
                        <LabelWithHint
                            label={t('elevated_autostart')}
                            hint={t('elevated_autostart_hint')}
                            hintKey="elevated_autostart"
                        />
                        <SettingSwitch
                            checked={elevatedAutostart}
                            disabled={!autoStart}
                            onChange={(e) => {
                                const enabled = e.target.checked;
                                applyToggle(enabled, setElevatedAutostart, "toggle_elevated_autostart", { enabled });
                            }}
                        />
                    </div>
                )}

                <div className="setting-item">
                    <LabelWithHint
                        label={t('silent_start')}
                        hint={t('silent_start_hint')}
                        hintKey="silent_start"
                    />
                    <SettingSwitch
                        checked={silentStart}
                        onChange={(e) => {
                            const enabled = e.target.checked;
                            applyToggle(enabled, setSilentStart, "set_silent_start", { enabled });
                        }}
                    />
                </div>

                {isWindowsPlatform() && (
                    <div className="setting-item">
                        <LabelWithHint
                            label={t('restart_as_admin')}
                            hint={t('restart_as_admin_hint')}
                            hintKey="restart_as_admin"
                        />
                        <button
                            type="button"
                            className="settings-action-button"
                            onClick={() => {
                                invoke("restart_as_admin").catch((error) => {
                                    console.error("Failed to restart as administrator:", error);
                                });
                            }}
                        >
                            {t('restart')}
                        </button>
                    </div>
                )}

                <h4 className="settings-section-title">{t('settings_section_window')}</h4>
                <div className="setting-item">
                    <LabelWithHint
                        label={t('hide_tray_icon')}
                        hint={t('hide_tray_icon_hint')}
                        hintKey="hide_tray_icon"
                    />
                    <SettingSwitch
                        checked={hideTrayIcon}
                        onChange={(e) => {
                            const val = e.target.checked;
                            applyToggle(val, setHideTrayIcon, "set_tray_visible", { visible: !val });
                        }}
                    />
                </div>

                {isMacPlatform() && (
                    <div className="setting-item">
                        <LabelWithHint
                            label={t('hide_dock_icon')}
                            hint={t('hide_dock_icon_hint')}
                            hintKey="hide_dock_icon"
                        />
                        <SettingSwitch
                            checked={hideDockIcon}
                            onChange={(e) => {
                                const val = e.target.checked;
                                applyToggle(val, setHideDockIcon, "set_dock_visible", { visible: !val });
                            }}
                        />
                    </div>
                )}

                <div className="setting-item">
                    <LabelWithHint
                        label={t('edge_docking')}
                        hint={t('edge_docking_hint')}
                        hintKey="edge_docking"
                    />
                    <SettingSwitch
                        checked={edgeDocking}
                        onChange={(e) => {
                            const val = e.target.checked;
                            applyToggle(val, setEdgeDocking, "set_edge_docking", { enabled: val });
                        }}
                    />
                </div>

                <div className="setting-item">
                    <LabelWithHint
                        label={t('follow_mouse')}
                        hint={t('follow_mouse_hint')}
                        hintKey="follow_mouse"
                    />
                    <SettingSwitch
                        checked={followMouse}
                        onChange={(e) => {
                            const val = e.target.checked;
                            applyToggle(val, setFollowMouse, "set_follow_mouse", { enabled: val });
                        }}
                    />
                </div>

                {isMacPlatform() && (
                    <MacAccessibilityPermission t={t} LabelWithHint={LabelWithHint} />
                )}

                <h4 className="settings-section-title">{t('settings_section_sound')}</h4>
                <div className="setting-item">
                    <LabelWithHint label={t('sound_effects')} hint={t('sound_effects_hint')} hintKey="sound_effects" />
                    <SettingSwitch
                        checked={soundEnabled}
                        onChange={(e) => {
                            const enabled = e.target.checked;
                            applyToggle(enabled, setSoundEnabled, "set_sound_enabled", { enabled }, () => {
                                if (enabled) playPreview("copy");
                            });
                        }}
                    />
                </div>
                <div className={`setting-item is-child${soundEnabled ? '' : ' is-disabled'}`}>
                    <LabelWithHint label={t('paste_sound')} hint={t('paste_sound_hint')} hintKey="paste_sound" />
                    <SettingSwitch
                        checked={pasteSoundEnabled}
                        disabled={!soundEnabled}
                        onChange={(e) => {
                            const enabled = e.target.checked;
                            applyToggle(enabled, setPasteSoundEnabled, "set_paste_sound_enabled", { enabled }, () => {
                                if (enabled && soundEnabled) playPreview("paste");
                            });
                        }}
                    />
                </div>
                <div className={`setting-item is-child${soundEnabled ? '' : ' is-disabled'}`}>
                    <div className="item-label-group">
                        <span className="item-label">{t('sound_volume')}</span>
                    </div>
                    <SettingRange
                        min={0}
                        max={1}
                        step={0.01}
                        value={soundVolume}
                        valueLabel={`${Math.round(soundVolume * 100)}%`}
                        aria-label={t('sound_volume')}
                        disabled={!soundEnabled}
                        onChange={(e) => setSoundVolume(parseFloat(e.target.value))}
                        onMouseUp={() => playPreview("copy")}
                        onTouchEnd={() => playPreview("copy")}
                    />
                </div>

                <h4 className="settings-section-title">{t('settings_section_interface')}</h4>
                <div className="setting-item">
                    <LabelWithHint
                        label={t('show_search_box')}
                        hint={t('show_search_box_hint')}
                        hintKey="show_search_box"
                    />
                    <SettingSwitch
                        checked={searchBoxPinned}
                        onChange={(e) => {
                            const enabled = e.target.checked;
                            setSearchBoxPinned(enabled);
                            setShowSearchBox(enabled);
                            saveAppSetting('show_search_box', String(enabled));
                        }}
                    />
                </div>
                <div className="setting-item">
                    <LabelWithHint
                        label={t('scroll_top_button')}
                        hint={t('scroll_top_button_hint')}
                        hintKey="scroll_top_button"
                    />
                    <SettingSwitch
                        checked={scrollTopButtonEnabled}
                        onChange={(e) => {
                            const enabled = e.target.checked;
                            setScrollTopButtonEnabled(enabled);
                            saveAppSetting('show_scroll_top_button', String(enabled));
                        }}
                    />
                </div>
                <div className="setting-item">
                    <LabelWithHint
                        label={t('emoji_panel_enabled')}
                        hint={t('emoji_panel_enabled_hint')}
                        hintKey="emoji_panel_enabled"
                    />
                    <SettingSwitch
                        checked={emojiPanelEnabled}
                        onChange={(e) => {
                            const enabled = e.target.checked;
                            setEmojiPanelEnabled(enabled);
                            saveAppSetting('emoji_panel_enabled', String(enabled));
                        }}
                    />
                </div>
                <div className="setting-item">
                    <LabelWithHint
                        label={t('tag_manager_enabled')}
                        hint={t('tag_manager_enabled_hint')}
                        hintKey="tag_manager_enabled"
                    />
                    <SettingSwitch
                        checked={tagManagerEnabled}
                        onChange={(e) => {
                            const enabled = e.target.checked;
                            setTagManagerEnabled(enabled);
                            saveAppSetting('tag_manager_enabled', String(enabled));
                        }}
                    />
                </div>
                <div className="setting-item">
                    <LabelWithHint
                        label={t('arrow_key_selection')}
                        hint={t('arrow_key_selection_hint')}
                        hintKey="arrow_key_selection"
                    />
                    <SettingSwitch
                        checked={arrowKeySelection}
                        onChange={(e) => {
                            const enabled = e.target.checked;
                            setArrowKeySelection(enabled);
                            saveAppSetting('arrow_key_selection', String(enabled));
                        }}
                    />
                </div>

                <h4 className="settings-section-title">{t('settings_section_performance')}</h4>
                <div className="setting-item">
                    <LabelWithHint
                        label={t('light_mode_enabled')}
                        hint={t('light_mode_enabled_hint')}
                        hintKey="light_mode_enabled"
                    />
                    <SettingSwitch
                        checked={lightModeEnabled}
                        onChange={(e) => {
                            const enabled = e.target.checked;
                            setLightModeEnabled(enabled);
                            saveAppSetting('light_mode_enabled', String(enabled));
                        }}
                    />
                </div>
                <div className={`setting-item is-child${lightModeEnabled ? '' : ' is-disabled'}`}>
                    <LabelWithHint
                        label={t('light_mode_idle_minutes')}
                        hint={t('light_mode_idle_minutes_hint')}
                        hintKey="light_mode_idle_minutes"
                    />
                    <select
                        className="search-input settings-field settings-field--sm"
                        value={lightModeIdleMinutes}
                        disabled={!lightModeEnabled}
                        onChange={(e) => {
                            const mins = Number.parseInt(e.target.value, 10);
                            const next = Number.isFinite(mins) ? mins : 15;
                            setLightModeIdleMinutes(next);
                            saveAppSetting('light_mode_idle_minutes', String(next));
                        }}
                    >
                        {[5, 15, 30, 60].map((mins) => (
                            <option key={mins} value={mins}>
                                {mins} {t('light_mode_minutes_unit')}
                            </option>
                        ))}
                    </select>
                </div>
            </div>
        )}
    </div>
);

export default GeneralSettingsGroup;
