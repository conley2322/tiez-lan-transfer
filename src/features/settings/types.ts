import type { AiProviderId, AiReasoningEffort } from "./aiProviders";

export interface AiProfile {
  id: string;
  /** Missing on profiles saved before providers existed; inferred from baseUrl. */
  provider?: AiProviderId;
  baseUrl: string;
  apiKey: string;
  model: string;
  enableThinking: boolean;
  reasoningEffort?: AiReasoningEffort;
  /** Token cap for providers that limit thinking by budget. */
  thinkingBudget?: number;
}

export type AppCleanupPolicyAction = "ignore" | "clean";

export interface AppCleanupPolicy {
  id: string;
  enabled: boolean;
  appName: string;
  appPath: string;
  action: AppCleanupPolicyAction;
  contentTypes: string[];
  cleanupRules: string;
}

export type EditableAiProfile = Omit<AiProfile, "id"> & { id?: string; isNew?: boolean };

export type AiProfileStatus = "loading" | "success" | "error" | "none";

export type AiProfileStatusMap = Record<string, AiProfileStatus>;

/** Every settings group starts collapsed each time the settings page opens. */
export const DEFAULT_COLLAPSED_GROUPS: Record<string, boolean> = {
  general: true,
  clipboard: true,
  appearance: true,
  sync: true,
  cloud_sync: true,
  e2e: true,
  ai: true,
  file_transfer: true,
  default_apps: true,
  secure_transfer: true,
  data: true
};
