import { describe, expect, it } from "vitest";
import {
  AI_PROVIDERS,
  inferAiProvider,
  getAiProvider,
} from "./aiProviders";

describe("AI Providers Presets and Inference", () => {
  it("defines standard industry AI providers with valid Base URLs and thinking switches", () => {
    expect(AI_PROVIDERS.length).toBeGreaterThanOrEqual(10);

    const deepseek = AI_PROVIDERS.find((p) => p.id === "deepseek");
    expect(deepseek).toBeDefined();
    expect(deepseek?.baseUrl).toBe("https://api.deepseek.com/v1");
    expect(deepseek?.thinking).toBe("toggle");
    expect(deepseek?.showsReasoning).toBe(true);

    const openai = AI_PROVIDERS.find((p) => p.id === "openai");
    expect(openai).toBeDefined();
    expect(openai?.thinking).toBe("effort");

    const ollama = AI_PROVIDERS.find((p) => p.id === "ollama");
    expect(ollama).toBeDefined();
    expect(ollama?.baseUrl).toContain("11434");
  });

  it("infers AI provider correctly from Base URLs", () => {
    expect(inferAiProvider("https://api.deepseek.com/v1")).toBe("deepseek");
    expect(inferAiProvider("https://api.openai.com/v1/chat/completions")).toBe("openai");
    expect(inferAiProvider("https://api.anthropic.com/v1")).toBe("anthropic");
    expect(inferAiProvider("https://dashscope.aliyuncs.com/compatible-mode/v1")).toBe("qwen");
    expect(inferAiProvider("https://open.bigmodel.cn/api/paas/v4")).toBe("zhipu");
    expect(inferAiProvider("http://localhost:11434/v1")).toBe("ollama");
    expect(inferAiProvider("https://custom-proxy.internal.net/v1")).toBe("custom");
  });

  it("resolves AI provider configuration from profile objects", () => {
    const explicitProfile = { provider: "deepseek" as const, baseUrl: "https://custom.mirror/v1" };
    expect(getAiProvider(explicitProfile).id).toBe("deepseek");

    const inferredProfile = { baseUrl: "https://api.moonshot.cn/v1" };
    expect(getAiProvider(inferredProfile).id).toBe("moonshot");
  });
});
