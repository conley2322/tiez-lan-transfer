import { describe, expect, it } from "vitest";
import { PRESET_AI_PROFILES, resolveTaskProfile, restoreAiProfiles } from "./aiPresets";
import type { AiProfile } from "./types";

const own = (id: string, apiKey: string, extra: Partial<AiProfile> = {}): AiProfile => ({
  id,
  provider: "openai",
  baseUrl: "https://api.openai.com/v1",
  apiKey,
  model: `model-${id}`,
  enableThinking: false,
  ...extra
});

describe("restoreAiProfiles", () => {
  it("keeps a deleted preset deleted", () => {
    const saved = [PRESET_AI_PROFILES[1], own("1", "sk")];
    expect(restoreAiProfiles(saved).map((p) => p.id)).toEqual(["ds_flash_v1", "1"]);
  });

  it("keeps the user's key on a preset", () => {
    const saved = [{ ...PRESET_AI_PROFILES[0], apiKey: "sk-ds" }];
    expect(restoreAiProfiles(saved)[0].apiKey).toBe("sk-ds");
  });

  it("replaces legacy LongCat presets with the DeepSeek ones", () => {
    const legacy = { ...own("lc_flash_v1", ""), model: "LongCat-Flash-Chat" };
    const ids = restoreAiProfiles([legacy, own("1", "sk")]).map((p) => p.id);
    expect(ids).toEqual(["ds_v4_pro_v1", "ds_flash_v1", "1"]);
  });

  it("drops malformed and duplicate entries", () => {
    const ids = restoreAiProfiles([null, { id: "x" }, own("1", "a"), own("1", "b")]).map((p) => p.id);
    expect(ids).toEqual(["1"]);
  });
});

describe("resolveTaskProfile", () => {
  const presets = PRESET_AI_PROFILES;

  it("moves a task off a keyless preset to the user's model with a key", () => {
    expect(resolveTaskProfile("ds_flash_v1", [...presets, own("1", "sk")])).toBe("1");
  });

  it("moves a task whose model was deleted", () => {
    expect(resolveTaskProfile("default", [...presets, own("1", "sk")])).toBe("1");
  });

  it("leaves a preset that has a key alone", () => {
    const withKey = [{ ...presets[0], apiKey: "sk-ds" }, presets[1], own("1", "sk")];
    expect(resolveTaskProfile("ds_v4_pro_v1", withKey)).toBe("ds_v4_pro_v1");
  });

  it("leaves a keyless model of the user's own alone (local servers need none)", () => {
    const local = own("local", "", { provider: "ollama", baseUrl: "http://localhost:11434/v1" });
    expect(resolveTaskProfile("local", [...presets, local, own("1", "sk")])).toBe("local");
  });

  it("keeps the assignment when no model has a key", () => {
    expect(resolveTaskProfile("ds_flash_v1", presets)).toBe("ds_flash_v1");
  });
});
