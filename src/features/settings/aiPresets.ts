import type { AiProfile } from "./types";

/** DeepSeek models offered on first run. They ship without a key. */
export const PRESET_AI_PROFILES: AiProfile[] = [
  {
    id: "ds_v4_pro_v1",
    provider: "deepseek",
    baseUrl: "https://api.deepseek.com/v1",
    apiKey: "",
    model: "deepseek-v4-pro",
    enableThinking: true
  },
  {
    id: "ds_flash_v1",
    provider: "deepseek",
    baseUrl: "https://api.deepseek.com/v1",
    apiKey: "",
    model: "deepseek-flash",
    enableThinking: false
  }
];

const PRESET_IDS = new Set(PRESET_AI_PROFILES.map((p) => p.id));

/** LongCat presets from older versions; they are replaced by the DeepSeek ones. */
export const LEGACY_PRESET_IDS = new Set(["lc_flash_v1", "lc_think_v1", "lc_think_2601_v1"]);
const LEGACY_PRESET_MODELS = new Set(["LongCat-Flash-Chat", "LongCat-Flash-Thinking", "LongCat-Flash-Thinking-2601"]);

const isAiProfile = (p: unknown): p is AiProfile => {
  if (!p || typeof p !== "object") return false;
  const r = p as Record<string, unknown>;
  return (
    typeof r.id === "string" &&
    typeof r.baseUrl === "string" &&
    typeof r.apiKey === "string" &&
    typeof r.model === "string" &&
    typeof r.enableThinking === "boolean"
  );
};

/**
 * The model list from a saved `ai_profiles` array. Presets are seeded only on
 * first run (no saved list) and when replacing the legacy LongCat presets, so a
 * preset the user deleted stays deleted.
 */
export const restoreAiProfiles = (saved: unknown[]): AiProfile[] => {
  const valid = saved.filter(isAiProfile);
  const hadLegacy = valid.some((p) => LEGACY_PRESET_IDS.has(p.id) || LEGACY_PRESET_MODELS.has(p.model));
  const kept = valid
    .filter((p) => !LEGACY_PRESET_IDS.has(p.id) && !LEGACY_PRESET_MODELS.has(p.model))
    .filter((p, index, arr) => arr.findIndex((q) => q.id === p.id) === index);
  if (!hadLegacy) return kept;
  const presets = PRESET_AI_PROFILES.map((preset) => {
    const own = kept.find((p) => p.id === preset.id);
    return own ? { ...preset, ...own } : preset;
  });
  return [...presets, ...kept.filter((p) => !PRESET_IDS.has(p.id))];
};

/**
 * Whether a task assigned to `id` has nothing to run on: the model is gone, or
 * it is a preset still without a key. Other keyless models may be local
 * servers that need none, so they are left alone.
 */
const isStrandedAssignment = (id: string, profiles: AiProfile[]): boolean => {
  const profile = profiles.find((p) => p.id === id);
  return !profile || (PRESET_IDS.has(profile.id) && !profile.apiKey.trim());
};

const isOwnModelWithKey = (profile: AiProfile): boolean =>
  !PRESET_IDS.has(profile.id) && profile.apiKey.trim() !== "";

/**
 * The model a task should be assigned to. A stranded task moves to the first
 * of the user's own models that has a key; anything else stays as chosen. The
 * presets are the default assignment, so without this a user who adds a model
 * would still hit "API Key is not set" until they re-assigned every task.
 */
export const resolveTaskProfile = (id: string, profiles: AiProfile[]): string => {
  if (!isStrandedAssignment(id, profiles)) return id;
  return profiles.find(isOwnModelWithKey)?.id ?? id;
};
