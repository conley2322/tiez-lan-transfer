export type AiProviderId =
  | "openai"
  | "anthropic"
  | "gemini"
  | "deepseek"
  | "qwen"
  | "siliconflow"
  | "zhipu"
  | "moonshot"
  | "openrouter"
  | "ollama"
  | "custom";

/**
 * How the profile modal presents deep thinking for a provider:
 * - effort: reasoning effort levels; the provider returns no visible reasoning
 * - budget: on/off, limited by the shared thinking budget
 * - toggle: on/off only
 * - model: decided by the chosen model, no switch
 */
export type AiThinkingControl = "effort" | "budget" | "toggle" | "model";

export type AiReasoningEffort = "low" | "medium" | "high";

export interface AiProviderPreset {
  id: AiProviderId;
  label: string;
  baseUrl: string;
  modelPlaceholder: string;
  thinking: AiThinkingControl;
  /** Whether the provider sends its reasoning text back for display. */
  showsReasoning: boolean;
}

export const AI_PROVIDERS: AiProviderPreset[] = [
  { id: "deepseek", label: "DeepSeek", baseUrl: "https://api.deepseek.com/v1", modelPlaceholder: "deepseek-chat", thinking: "toggle", showsReasoning: true },
  { id: "openai", label: "OpenAI", baseUrl: "https://api.openai.com/v1", modelPlaceholder: "gpt-5-mini", thinking: "effort", showsReasoning: false },
  { id: "anthropic", label: "Anthropic Claude", baseUrl: "https://api.anthropic.com/v1", modelPlaceholder: "claude-sonnet-4-5", thinking: "budget", showsReasoning: true },
  { id: "gemini", label: "Google Gemini", baseUrl: "https://generativelanguage.googleapis.com/v1beta", modelPlaceholder: "gemini-2.5-flash", thinking: "budget", showsReasoning: true },
  { id: "qwen", label: "通义千问 Qwen", baseUrl: "https://dashscope.aliyuncs.com/compatible-mode/v1", modelPlaceholder: "qwen-plus", thinking: "budget", showsReasoning: true },
  { id: "zhipu", label: "智谱 GLM", baseUrl: "https://open.bigmodel.cn/api/paas/v4", modelPlaceholder: "glm-4.6", thinking: "toggle", showsReasoning: true },
  { id: "moonshot", label: "Kimi (Moonshot)", baseUrl: "https://api.moonshot.cn/v1", modelPlaceholder: "kimi-k2-thinking", thinking: "model", showsReasoning: true },
  { id: "siliconflow", label: "硅基流动 SiliconFlow", baseUrl: "https://api.siliconflow.cn/v1", modelPlaceholder: "Qwen/Qwen3-8B", thinking: "budget", showsReasoning: true },
  { id: "openrouter", label: "OpenRouter", baseUrl: "https://openrouter.ai/api/v1", modelPlaceholder: "openai/gpt-5-mini", thinking: "budget", showsReasoning: true },
  { id: "ollama", label: "Ollama", baseUrl: "http://localhost:11434/v1", modelPlaceholder: "qwen3:8b", thinking: "effort", showsReasoning: true },
  { id: "custom", label: "", baseUrl: "", modelPlaceholder: "model-name", thinking: "budget", showsReasoning: true }
];

const HOST_HINTS: [string, AiProviderId][] = [
  ["api.openai.com", "openai"],
  ["api.anthropic.com", "anthropic"],
  ["generativelanguage.googleapis.com", "gemini"],
  ["api.deepseek.com", "deepseek"],
  ["dashscope", "qwen"],
  ["siliconflow", "siliconflow"],
  ["bigmodel.cn", "zhipu"],
  ["z.ai", "zhipu"],
  ["moonshot", "moonshot"],
  ["openrouter.ai", "openrouter"],
  [":11434", "ollama"]
];

/** Mirrors the backend inference for profiles saved before providers existed. */
export const inferAiProvider = (baseUrl: string): AiProviderId => {
  const url = baseUrl.toLowerCase();
  return HOST_HINTS.find(([needle]) => url.includes(needle))?.[1] ?? "custom";
};

export const getAiProvider = (profile: { provider?: AiProviderId; baseUrl: string }): AiProviderPreset => {
  const id = profile.provider ?? inferAiProvider(profile.baseUrl);
  return AI_PROVIDERS.find((p) => p.id === id) ?? AI_PROVIDERS[AI_PROVIDERS.length - 1];
};
