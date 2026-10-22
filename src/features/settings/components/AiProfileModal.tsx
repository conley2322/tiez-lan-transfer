import AppModal from "../../../shared/components/AppModal";
import { AI_PROVIDERS, getAiProvider } from "../aiProviders";
import type { AiProviderId, AiReasoningEffort } from "../aiProviders";
import type { EditableAiProfile } from "../types";

interface AiProfileModalProps {
    editingProfile: EditableAiProfile | null;
    t: (key: string) => string;
    onClose: () => void;
    onSave: (profile: EditableAiProfile) => void;
    setEditingProfile: (val: EditableAiProfile) => void;
}

const PRESET_URLS = new Set(AI_PROVIDERS.map((p) => p.baseUrl));
const MIN_THINKING_BUDGET = 1024;
const MAX_THINKING_BUDGET = 32000;

const AiProfileModal = ({ editingProfile, t, onClose, onSave, setEditingProfile }: AiProfileModalProps) => {
    const provider = editingProfile ? getAiProvider(editingProfile) : null;

    const changeProvider = (id: AiProviderId) => {
        if (!editingProfile) return;
        const next = AI_PROVIDERS.find((p) => p.id === id)!;
        // Replace the URL only when it is still a preset default, keeping hand-entered proxies.
        const keepUrl = editingProfile.baseUrl.trim() && !PRESET_URLS.has(editingProfile.baseUrl.trim());
        setEditingProfile({
            ...editingProfile,
            provider: id,
            baseUrl: keepUrl ? editingProfile.baseUrl : next.baseUrl,
            enableThinking: next.thinking === "model" ? false : editingProfile.enableThinking
        });
    };

    return (
        <AppModal
            open={Boolean(editingProfile)}
            onClose={onClose}
            panelClassName="modal-panel modal-form-dialog ai-profile-dialog"
            ariaLabel={editingProfile?.isNew ? t("ai_add_model") : t("ai_edit_model")}
        >
            {editingProfile && provider && (
                <>
                    <div className="modal-form-heading">
                        <h3 className="modal-form-title">{editingProfile.isNew ? t("ai_add_model") : t("ai_edit_model")}</h3>
                    </div>

                    <div className="modal-form-fields">
                        <label className="modal-form-field">
                            <span className="modal-field-label">{t("ai_provider")}</span>
                            <select
                                className="search-input modal-form-control"
                                value={provider.id}
                                onChange={(e) => changeProvider(e.target.value as AiProviderId)}
                            >
                                {AI_PROVIDERS.map((p) => (
                                    <option key={p.id} value={p.id}>{p.label || t("ai_provider_custom")}</option>
                                ))}
                            </select>
                        </label>
                        <label className="modal-form-field">
                            <span className="modal-field-label">{t("ai_base_url")}</span>
                            <input
                                className="search-input modal-form-control"
                                value={editingProfile.baseUrl}
                                placeholder={provider.baseUrl || "https://example.com/v1"}
                                onChange={(e) => setEditingProfile({ ...editingProfile, baseUrl: e.target.value })}
                            />
                        </label>
                        <label className="modal-form-field">
                            <span className="modal-field-label">{t("ai_api_key")}</span>
                            <input
                                className="search-input modal-form-control"
                                type="password"
                                value={editingProfile.apiKey}
                                placeholder={provider.id === "ollama" ? t("ai_api_key_optional") : undefined}
                                onChange={(e) => setEditingProfile({ ...editingProfile, apiKey: e.target.value })}
                            />
                        </label>
                        <label className="modal-form-field">
                            <span className="modal-field-label">{t("ai_model_id")}</span>
                            <input
                                className="search-input modal-form-control"
                                value={editingProfile.model}
                                onChange={(e) => setEditingProfile({ ...editingProfile, model: e.target.value })}
                                placeholder={provider.modelPlaceholder}
                            />
                        </label>
                    </div>
                    {provider.thinking !== "model" && (
                        <div className="modal-field-row modal-form-toggle-row">
                            <span style={{ fontSize: 'var(--font-size-base)', fontWeight: 600 }}>{t("ai_thinking_enabled")}</span>
                            <label className="switch">
                                <input
                                    className="cb"
                                    type="checkbox"
                                    checked={editingProfile.enableThinking}
                                    onChange={(e) => setEditingProfile({ ...editingProfile, enableThinking: e.target.checked })}
                                />
                                <div className="toggle"><div className="left" /><div className="right" /></div>
                            </label>
                        </div>
                    )}
                    <div className="modal-form-fields">
                        {provider.thinking === "effort" && editingProfile.enableThinking && (
                            <label className="modal-form-field">
                                <span className="modal-field-label">{t("ai_reasoning_effort")}</span>
                                <select
                                    className="search-input modal-form-control"
                                    value={editingProfile.reasoningEffort ?? "medium"}
                                    onChange={(e) => setEditingProfile({ ...editingProfile, reasoningEffort: e.target.value as AiReasoningEffort })}
                                >
                                    <option value="low">{t("ai_effort_low")}</option>
                                    <option value="medium">{t("ai_effort_medium")}</option>
                                    <option value="high">{t("ai_effort_high")}</option>
                                </select>
                            </label>
                        )}
                        {provider.thinking === "budget" && editingProfile.enableThinking && (
                            <label className="modal-form-field">
                                <span className="modal-field-label">{t("ai_thinking_budget")}</span>
                                <input
                                    className="search-input modal-form-control"
                                    type="number"
                                    min={MIN_THINKING_BUDGET}
                                    max={MAX_THINKING_BUDGET}
                                    step="512"
                                    // Empty while typing (placeholder shows the default); clamped on blur.
                                    value={editingProfile.thinkingBudget ?? ""}
                                    placeholder={String(MIN_THINKING_BUDGET)}
                                    onChange={(e) => setEditingProfile({ ...editingProfile, thinkingBudget: Number(e.target.value) || undefined })}
                                    onBlur={() => setEditingProfile({
                                        ...editingProfile,
                                        thinkingBudget: Math.min(
                                            MAX_THINKING_BUDGET,
                                            Math.max(editingProfile.thinkingBudget ?? 0, MIN_THINKING_BUDGET)
                                        )
                                    })}
                                />
                            </label>
                        )}
                        <p className="modal-form-hint">
                            {t(`ai_thinking_hint_${provider.thinking}`)}
                            {!provider.showsReasoning && ` ${t("ai_reasoning_hidden_hint")}`}
                        </p>
                    </div>
                    <div className="modal-form-actions split">
                        <button type="button" className="modal-button" onClick={onClose}>{t("cancel")}</button>
                        <button
                            type="button"
                            className="modal-button primary"
                            disabled={!editingProfile.baseUrl.trim()}
                            title={editingProfile.baseUrl.trim() ? undefined : t("ai_url_required")}
                            onClick={() => onSave(editingProfile)}
                        >
                            {t("ai_save_profile")}
                        </button>
                    </div>
                </>
            )}
        </AppModal>
    );
};

export default AiProfileModal;
