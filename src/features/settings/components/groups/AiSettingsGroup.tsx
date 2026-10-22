import { ChevronDown, ChevronRight, Edit2, RotateCcw, Trash2 } from "lucide-react";
import { getAiProvider } from "../../aiProviders";
import type { AiProfile, AiProfileStatusMap, EditableAiProfile } from "../../types";
import { SettingSwitch } from "../SettingControls";

interface AiSettingsGroupProps {
    t: (key: string) => string;
    collapsed: boolean;
    onToggle: () => void;
    aiEnabled: boolean;
    setAiEnabled: (val: boolean) => void;
    saveSetting: (key: string, val: string) => void;
    aiProfiles: AiProfile[];
    profileStatuses: AiProfileStatusMap;
    checkModelStatus: (profile: AiProfile) => void;
    setEditingProfile: (profile: EditableAiProfile) => void;
    handleDeleteProfile: (id: string) => void;
    aiAssignedProfileTask: string;
    setAiAssignedProfileTask: (id: string) => void;
    aiAssignedProfileMouthpiece: string;
    setAiAssignedProfileMouthpiece: (id: string) => void;
    aiAssignedProfileTranslate: string;
    setAiAssignedProfileTranslate: (id: string) => void;
    aiTargetLang: string;
    setAiTargetLang: (val: string) => void;
    theme: string;
}

const AiSettingsGroup = ({
    t,
    collapsed,
    onToggle,
    aiEnabled,
    setAiEnabled,
    saveSetting,
    aiProfiles,
    profileStatuses,
    checkModelStatus,
    setEditingProfile,
    handleDeleteProfile,
    aiAssignedProfileTask,
    setAiAssignedProfileTask,
    aiAssignedProfileMouthpiece,
    setAiAssignedProfileMouthpiece,
    aiAssignedProfileTranslate,
    setAiAssignedProfileTranslate,
    aiTargetLang,
    setAiTargetLang
}: AiSettingsGroupProps) => (
    <div className={`settings-group ${collapsed ? 'collapsed' : ''}`}>
        <button type="button" className="group-header" aria-expanded={!collapsed} onClick={onToggle}>
            <h3>{t('ai_settings')}</h3>
            {collapsed ? <ChevronRight size={16} /> : <ChevronDown size={16} />}
        </button>
        {!collapsed && (
            <div className="group-content">
                <div className="setting-item">
                    <div className="item-label-group">
                        <span className="item-label">{t('enable_ai')}</span>
                    </div>
                    <SettingSwitch
                        checked={aiEnabled}
                        onChange={(e) => {
                            const val = e.target.checked;
                            setAiEnabled(val);
                            saveSetting('ai_enabled', String(val));
                        }}
                    />
                </div>

                {aiEnabled && (
                    <>
                        <div className="settings-section-header">
                            <h4 className="settings-section-title">{t('ai_model_library')}</h4>
                            <button
                                type="button"
                                className="settings-action-button settings-action-button--small"
                                onClick={() => setEditingProfile({ isNew: true, provider: 'deepseek', baseUrl: 'https://api.deepseek.com/v1', apiKey: '', model: '', enableThinking: false })}
                            >
                                {t('ai_add_model')}
                            </button>
                        </div>

                        <div className="ai-profile-list">
                            {aiProfiles.map(profile => {
                                const profileStatus = profileStatuses[profile.id];
                                const statusTone = profileStatus === 'success' ? 'is-connected'
                                    : profileStatus === 'error' ? 'is-disconnected'
                                        : profileStatus === 'loading' ? 'is-connecting' : '';
                                return (
                                    <div key={profile.id} className="ai-profile-card">
                                        <span className={`settings-status-dot ${statusTone}`} title={profileStatus || 'Unknown'} />
                                        <div className="ai-profile-text">
                                            <div className="ai-profile-name">{profile.model}</div>
                                            <div className="ai-profile-sub">
                                                {getAiProvider(profile).label || t('ai_provider_custom')}
                                                {profile.enableThinking && getAiProvider(profile).thinking !== 'model' && ` · ${t('ai_thinking_enabled')}`}
                                            </div>
                                        </div>
                                        <div className="ai-profile-actions">
                                            <button type="button" className="btn-icon" onClick={() => checkModelStatus(profile)} title={t('ai_check_connection')}>
                                                <RotateCcw size={13} className={profileStatus === 'loading' ? 'animate-spin' : ''} />
                                            </button>
                                            <button type="button" className="btn-icon" onClick={() => setEditingProfile(profile)}><Edit2 size={13} /></button>
                                            <button type="button" className="btn-icon is-danger" onClick={() => handleDeleteProfile(profile.id)}><Trash2 size={13} /></button>
                                        </div>
                                    </div>
                                );
                            })}
                        </div>

                        <h4 className="settings-section-title">{t('ai_strategy_settings')}</h4>

                        {[
                            { label: t('ai_strategy_task'), value: aiAssignedProfileTask, setter: setAiAssignedProfileTask, key: 'ai_assigned_profile_task' },
                            { label: t('ai_strategy_mouthpiece'), value: aiAssignedProfileMouthpiece, setter: setAiAssignedProfileMouthpiece, key: 'ai_assigned_profile_mouthpiece' },
                        ].map(strategy => (
                            <div className="setting-item" key={strategy.key}>
                                <div className="item-label-group">
                                    <span className="item-label">{strategy.label}</span>
                                </div>
                                <select
                                    className="search-input settings-field"
                                    value={strategy.value}
                                    onChange={e => {
                                        strategy.setter(e.target.value);
                                        saveSetting(strategy.key, e.target.value);
                                    }}
                                >
                                    <option value="none" disabled>{aiProfiles.length > 0 ? t('select_a_profile') : t('add_profile_first')}</option>
                                    {aiProfiles.map(p => (
                                        <option key={p.id} value={p.id}>{p.model}</option>
                                    ))}
                                </select>
                            </div>
                        ))}

                        <div className="setting-item">
                            <div className="item-label-group">
                                <span className="item-label">{t('ai_strategy_translate')}</span>
                            </div>
                            <select
                                className="search-input settings-field"
                                value={aiAssignedProfileTranslate}
                                onChange={e => {
                                    setAiAssignedProfileTranslate(e.target.value);
                                    saveSetting('ai_assigned_profile_translate', e.target.value);
                                }}
                            >
                                <option value="none" disabled>{aiProfiles.length > 0 ? t('select_a_profile') : t('add_profile_first')}</option>
                                {aiProfiles.map(p => (
                                    <option key={p.id} value={p.id}>{p.model}</option>
                                ))}
                            </select>
                        </div>

                        <div className="setting-item is-child">
                            <div className="item-label-group">
                                <span className="item-label">{t('ai_target_lang')}</span>
                            </div>
                            <select
                                className="search-input settings-field"
                                value={aiTargetLang}
                                onChange={e => {
                                    setAiTargetLang(e.target.value);
                                    saveSetting('ai_target_lang', e.target.value);
                                }}
                            >
                                <option value="auto_zh_en">{t('lang_auto_zh_en')}</option>
                                <option value="zh">{t('lang_zh')}</option>
                                <option value="en">{t('lang_en')}</option>
                                <option value="ja">{t('lang_ja')}</option>
                                <option value="de">{t('lang_de')}</option>
                                <option value="fr">{t('lang_fr')}</option>
                            </select>
                        </div>
                    </>
                )}
            </div>
        )}
    </div>
);

export default AiSettingsGroup;
