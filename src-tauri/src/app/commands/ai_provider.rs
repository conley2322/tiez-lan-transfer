use serde::Serialize;
use serde_json::{json, Map, Value};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Protocol {
    OpenAi,
    Anthropic,
    Gemini,
}

/// How a provider expects deep thinking to be requested.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Thinking {
    /// OpenAI reasoning models: `reasoning_effort`, `max_completion_tokens`,
    /// and no sampling parameters.
    OpenAiEffort,
    /// Ollama's OpenAI-compatible endpoint accepts `reasoning_effort` but keeps
    /// the classic sampling parameters.
    Effort,
    DeepSeek,
    Zhipu,
    OpenRouter,
    /// Qwen / SiliconFlow style `enable_thinking` + `thinking_budget`.
    EnableFlag,
    /// Thinking is chosen by the model name (e.g. Kimi thinking models).
    ModelChosen,
    Anthropic,
    Gemini,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Provider {
    pub protocol: Protocol,
    thinking: Thinking,
    pub requires_key: bool,
}

impl Provider {
    pub fn resolve(id: &str, base_url: &str) -> Self {
        let id = match id.trim() {
            "" => infer_provider_id(base_url),
            other => other,
        };
        let (protocol, thinking) = match id {
            "openai" => (Protocol::OpenAi, Thinking::OpenAiEffort),
            "anthropic" => (Protocol::Anthropic, Thinking::Anthropic),
            "gemini" => (Protocol::Gemini, Thinking::Gemini),
            "deepseek" => (Protocol::OpenAi, Thinking::DeepSeek),
            "zhipu" => (Protocol::OpenAi, Thinking::Zhipu),
            "openrouter" => (Protocol::OpenAi, Thinking::OpenRouter),
            "moonshot" => (Protocol::OpenAi, Thinking::ModelChosen),
            "ollama" => (Protocol::OpenAi, Thinking::Effort),
            // qwen, siliconflow and custom endpoints keep the original request shape.
            _ => (Protocol::OpenAi, Thinking::EnableFlag),
        };
        Self {
            protocol,
            thinking,
            // Servers on this machine or the local network (Ollama, LM Studio,
            // local gateways) usually take no key (#114).
            requires_key: id != "ollama" && !is_local_host(base_url),
        }
    }
}

/// Host and port of a base URL, lowercased, e.g. "localhost:8555".
fn authority_of(base_url: &str) -> String {
    let url = base_url.trim().to_ascii_lowercase();
    url.split_once("://")
        .map_or(url.as_str(), |(_, rest)| rest)
        .split(['/', '?', '#'])
        .next()
        .unwrap_or_default()
        .rsplit('@')
        .next()
        .unwrap_or_default()
        .to_string()
}

fn host_of(authority: &str) -> &str {
    match authority.strip_prefix('[') {
        // IPv6 literal: "[::1]:8555"
        Some(rest) => rest.split(']').next().unwrap_or_default(),
        None => authority.split(':').next().unwrap_or_default(),
    }
}

/// Loopback, private-network or `.local` host.
fn is_local_host(base_url: &str) -> bool {
    let authority = authority_of(base_url);
    let host = host_of(&authority);
    if host == "localhost" || host.ends_with(".localhost") || host.ends_with(".local") {
        return true;
    }
    match host.parse::<std::net::IpAddr>() {
        Ok(std::net::IpAddr::V4(ip)) => ip.is_loopback() || ip.is_private() || ip.is_link_local(),
        Ok(std::net::IpAddr::V6(ip)) => ip.is_loopback(),
        Err(_) => false,
    }
}

pub fn infer_provider_id(base_url: &str) -> &'static str {
    // Match against the host (and port) only, so a path can't fake a provider.
    let authority = authority_of(base_url);
    let authority = authority.as_str();
    let host = host_of(authority);
    // Domains match the host or a subdomain of it ("z.ai" is not "xyz.ai");
    // the rest are distinctive words or a port.
    let matches = |needle: &str| {
        if needle.starts_with(':') {
            authority.ends_with(needle)
        } else if needle.contains('.') {
            host == needle || host.ends_with(&format!(".{needle}"))
        } else {
            host.contains(needle)
        }
    };
    const HOSTS: [(&str, &str); 11] = [
        ("api.openai.com", "openai"),
        ("api.anthropic.com", "anthropic"),
        ("generativelanguage.googleapis.com", "gemini"),
        ("api.deepseek.com", "deepseek"),
        ("dashscope", "qwen"),
        ("siliconflow", "siliconflow"),
        ("bigmodel.cn", "zhipu"),
        ("z.ai", "zhipu"),
        ("moonshot", "moonshot"),
        ("openrouter.ai", "openrouter"),
        (":11434", "ollama"),
    ];
    HOSTS
        .iter()
        .find(|(needle, _)| matches(needle))
        .map(|(_, id)| *id)
        .unwrap_or("custom")
}

pub struct AiRequest<'a> {
    pub system: &'a str,
    pub user: &'a str,
    pub max_tokens: i32,
    pub temperature: f32,
    pub presence_penalty: f32,
    pub thinking: bool,
    pub thinking_budget: i32,
    pub reasoning_effort: &'a str,
}

pub struct HttpCall {
    pub url: String,
    pub headers: Vec<(&'static str, String)>,
    pub body: Value,
}

#[derive(Debug, Default, Serialize, PartialEq)]
pub struct AiReply {
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning: Option<String>,
}

pub fn build_call(
    provider: &Provider,
    base_url: &str,
    api_key: &str,
    model: &str,
    req: &AiRequest,
) -> HttpCall {
    match provider.protocol {
        Protocol::OpenAi => build_openai(provider.thinking, base_url, api_key, model, req),
        Protocol::Anthropic => build_anthropic(base_url, api_key, model, req),
        Protocol::Gemini => build_gemini(base_url, api_key, model, req),
    }
}

fn build_openai(
    thinking: Thinking,
    base_url: &str,
    api_key: &str,
    model: &str,
    req: &AiRequest,
) -> HttpCall {
    let mut body = Map::new();
    body.insert("model".into(), json!(model));
    body.insert(
        "messages".into(),
        json!([
            { "role": "system", "content": req.system },
            { "role": "user", "content": req.user }
        ]),
    );

    if thinking == Thinking::OpenAiEffort {
        // Reasoning models reject max_tokens, temperature and presence_penalty.
        body.insert("max_completion_tokens".into(), json!(req.max_tokens));
    } else {
        body.insert("max_tokens".into(), json!(req.max_tokens));
        body.insert("temperature".into(), json!(req.temperature));
        if thinking != Thinking::Zhipu {
            body.insert("presence_penalty".into(), json!(req.presence_penalty));
        }
    }

    match thinking {
        Thinking::OpenAiEffort | Thinking::Effort if req.thinking => {
            body.insert("reasoning_effort".into(), json!(req.reasoning_effort));
        }
        // DeepSeek models think unless told not to, so "off" must be sent.
        Thinking::DeepSeek | Thinking::Zhipu => {
            let kind = if req.thinking { "enabled" } else { "disabled" };
            body.insert("thinking".into(), json!({ "type": kind }));
        }
        Thinking::OpenRouter if req.thinking => {
            body.insert(
                "reasoning".into(),
                json!({ "max_tokens": req.thinking_budget }),
            );
        }
        Thinking::EnableFlag if req.thinking => {
            body.insert("enable_thinking".into(), json!(true));
            body.insert("thinking_budget".into(), json!(req.thinking_budget));
        }
        _ => {}
    }

    let referer = std::env::var("AI_HTTP_REFERER")
        .unwrap_or_else(|_| "https://github.com/jimuzhe/tiez".to_string());
    let mut headers = vec![
        ("HTTP-Referer", referer),
        ("X-Title", "TieZ Clipboard".to_string()),
    ];
    if !api_key.trim().is_empty() {
        headers.push(("Authorization", format!("Bearer {}", api_key.trim())));
    }

    HttpCall {
        url: format!("{}/chat/completions", base_url),
        headers,
        body: Value::Object(body),
    }
}

/// Claude models up to 4.6 take a fixed thinking budget and a temperature.
/// Later ones (4.7, 4.8, the 5 family, Fable) answer both with a 400: thinking
/// is adaptive and its depth is set by effort.
fn anthropic_takes_budget(model: &str) -> bool {
    let model = model.trim().to_ascii_lowercase();
    // Proxies may prefix the vendor, e.g. "anthropic/claude-sonnet-4-5".
    let model = model.rsplit('/').next().unwrap_or_default();
    if ["claude-3", "claude-2", "claude-instant"]
        .iter()
        .any(|old| model.starts_with(old))
    {
        return true;
    }
    ["opus", "sonnet", "haiku"].iter().any(|family| {
        let Some(rest) = model.strip_prefix(&format!("claude-{family}-4")) else {
            return false;
        };
        // "" is 4.0, as is an 8-digit snapshot date; otherwise the minor version.
        let minor = rest.strip_prefix('-').unwrap_or(rest);
        let minor = minor.split(['-', '@']).next().unwrap_or_default();
        minor.is_empty() || minor.len() == 8 || minor.parse::<u32>().is_ok_and(|m| m <= 6)
    })
}

fn build_anthropic(base_url: &str, api_key: &str, model: &str, req: &AiRequest) -> HttpCall {
    let mut body = json!({
        "model": model,
        "system": req.system,
        "messages": [{ "role": "user", "content": req.user }],
        "max_tokens": req.max_tokens,
    });
    if !anthropic_takes_budget(model) {
        if req.thinking {
            // Summarized, so the reasoning can be shown; the default is empty text.
            body["thinking"] = json!({ "type": "adaptive", "display": "summarized" });
        } else {
            // Some models always think; low effort keeps that short.
            body["output_config"] = json!({ "effort": "low" });
        }
    } else if req.thinking {
        // Extended thinking does not allow a custom temperature.
        body["thinking"] = json!({ "type": "enabled", "budget_tokens": req.thinking_budget });
    } else {
        body["temperature"] = json!(req.temperature);
    }
    HttpCall {
        url: format!("{}/messages", base_url),
        headers: vec![
            ("x-api-key", api_key.trim().to_string()),
            ("anthropic-version", "2023-06-01".to_string()),
        ],
        body,
    }
}

const GEMINI_THINKING_HEADROOM: i32 = 8192;

fn build_gemini(base_url: &str, api_key: &str, model: &str, req: &AiRequest) -> HttpCall {
    let model = model.trim().trim_start_matches("models/");
    // Gemini models think by default, and those tokens count against
    // maxOutputTokens. With thinking off: 2.5 Flash models accept a budget of 0;
    // others (2.5 Pro, newer models) can't switch it off, so give them headroom
    // for it instead of letting it starve the reply.
    let can_disable = model.starts_with("gemini-2.5-flash");
    let max_tokens = if req.thinking || can_disable {
        req.max_tokens
    } else {
        req.max_tokens.saturating_add(GEMINI_THINKING_HEADROOM)
    };
    let mut generation = json!({
        "maxOutputTokens": max_tokens,
        "temperature": req.temperature,
    });
    if req.thinking {
        generation["thinkingConfig"] = json!({
            "thinkingBudget": req.thinking_budget,
            "includeThoughts": true,
        });
    } else if can_disable {
        generation["thinkingConfig"] = json!({ "thinkingBudget": 0 });
    }
    HttpCall {
        url: format!("{}/models/{}:generateContent", base_url, model),
        headers: vec![("x-goog-api-key", api_key.trim().to_string())],
        body: json!({
            "systemInstruction": { "parts": [{ "text": req.system }] },
            "contents": [{ "role": "user", "parts": [{ "text": req.user }] }],
            "generationConfig": generation,
        }),
    }
}

pub fn parse_reply(protocol: Protocol, body: &Value) -> AiReply {
    let (content, reasoning) = match protocol {
        Protocol::OpenAi => {
            let message = &body["choices"][0]["message"];
            let content = text_of(&message["content"]);
            let reasoning = [&message["reasoning_content"], &message["reasoning"]]
                .into_iter()
                .find_map(|v| v.as_str().filter(|s| !s.trim().is_empty()))
                .map(str::to_string);
            (content, reasoning)
        }
        Protocol::Anthropic => {
            let blocks = body["content"].as_array().cloned().unwrap_or_default();
            let collect = |kind: &str, field: &str| {
                blocks
                    .iter()
                    .filter(|b| b["type"] == kind)
                    .filter_map(|b| b[field].as_str())
                    .collect::<Vec<_>>()
                    .join("\n")
            };
            (collect("text", "text"), Some(collect("thinking", "thinking")))
        }
        Protocol::Gemini => {
            let parts = body["candidates"][0]["content"]["parts"]
                .as_array()
                .cloned()
                .unwrap_or_default();
            let collect = |thought: bool| {
                parts
                    .iter()
                    .filter(|p| p["thought"].as_bool().unwrap_or(false) == thought)
                    .filter_map(|p| p["text"].as_str())
                    .collect::<Vec<_>>()
                    .join("")
            };
            (collect(false), Some(collect(true)))
        }
    };

    // Some open models inline their reasoning as a leading <think> block.
    let (content, inline) = split_think_block(&content);
    let reasoning = reasoning
        .filter(|s| !s.trim().is_empty())
        .or(inline)
        .map(|s| s.trim().to_string());
    AiReply { content, reasoning }
}

fn text_of(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        Value::Array(parts) => parts
            .iter()
            .filter_map(|p| p["text"].as_str())
            .collect::<Vec<_>>()
            .join(""),
        _ => String::new(),
    }
}

fn split_think_block(content: &str) -> (String, Option<String>) {
    let trimmed = content.trim_start();
    if let Some(rest) = trimmed.strip_prefix("<think>") {
        if let Some(end) = rest.find("</think>") {
            let thought = rest[..end].trim();
            let answer = rest[end + "</think>".len()..].to_string();
            let thought = (!thought.is_empty()).then(|| thought.to_string());
            return (answer, thought);
        }
    }
    (content.to_string(), None)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(thinking: bool) -> AiRequest<'static> {
        AiRequest {
            system: "sys",
            user: "hi",
            max_tokens: 3000,
            temperature: 0.7,
            presence_penalty: 0.6,
            thinking,
            thinking_budget: 1024,
            reasoning_effort: "high",
        }
    }

    fn call(id: &str, base: &str, thinking: bool) -> HttpCall {
        build_call(&Provider::resolve(id, base), base, "k", "m", &request(thinking))
    }

    #[test]
    fn infers_provider_from_legacy_base_url() {
        assert_eq!(infer_provider_id("https://api.deepseek.com/v1"), "deepseek");
        assert_eq!(infer_provider_id("http://localhost:11434/v1"), "ollama");
        assert_eq!(infer_provider_id("https://example.com/v1"), "custom");
        assert_eq!(infer_provider_id("https://api.z.ai/api/paas/v4"), "zhipu");
        assert_eq!(infer_provider_id("https://xyz.ai/v1"), "custom");
        assert_eq!(infer_provider_id("https://proxy.example.com/api.openai.com/v1"), "custom");
        assert!(!Provider::resolve("", "http://127.0.0.1:11434/v1").requires_key);
    }

    #[test]
    fn local_servers_need_no_key() {
        for local in [
            "http://127.0.0.1:8555/zen/v1",
            "http://localhost:8555/zen/v1",
            "http://[::1]:8555/v1",
            "http://192.168.1.20:1234/v1",
            "http://nas.local:8080/v1",
        ] {
            assert!(!Provider::resolve("custom", local).requires_key, "{local}");
        }
        assert!(Provider::resolve("custom", "https://example.com/v1").requires_key);
        assert!(Provider::resolve("openai", "https://api.openai.com/v1").requires_key);
    }

    #[test]
    fn openai_reasoning_uses_effort_without_sampling() {
        let c = call("openai", "https://api.openai.com/v1", true);
        assert_eq!(c.url, "https://api.openai.com/v1/chat/completions");
        assert_eq!(c.body["reasoning_effort"], "high");
        assert_eq!(c.body["max_completion_tokens"], 3000);
        assert!(c.body.get("max_tokens").is_none());
        assert!(c.body.get("temperature").is_none());
    }

    #[test]
    fn provider_specific_thinking_switches() {
        let ds = call("deepseek", "https://api.deepseek.com/v1", true);
        assert_eq!(ds.body["thinking"]["type"], "enabled");
        assert!(ds.body.get("enable_thinking").is_none());
        let ds_off = call("deepseek", "https://api.deepseek.com/v1", false);
        assert_eq!(ds_off.body["thinking"]["type"], "disabled");

        let glm = call("zhipu", "https://open.bigmodel.cn/api/paas/v4", false);
        assert_eq!(glm.body["thinking"]["type"], "disabled");

        let or = call("openrouter", "https://openrouter.ai/api/v1", true);
        assert_eq!(or.body["reasoning"]["max_tokens"], 1024);

        let legacy = call("custom", "https://example.com/v1", true);
        assert_eq!(legacy.body["enable_thinking"], true);
        assert_eq!(legacy.body["thinking_budget"], 1024);

        let kimi = call("moonshot", "https://api.moonshot.cn/v1", true);
        assert!(kimi.body.get("thinking").is_none());
        assert!(kimi.body.get("enable_thinking").is_none());
    }

    #[test]
    fn anthropic_thinking_follows_model_generation() {
        for old in [
            "claude-sonnet-4-5",
            "claude-haiku-4-5",
            "claude-sonnet-4-6",
            "claude-opus-4-1-20250805",
            "claude-sonnet-4-20250514",
            "claude-sonnet-4",
            "claude-3-5-sonnet-latest",
            "anthropic/claude-sonnet-4-5",
        ] {
            assert!(anthropic_takes_budget(old), "{old}");
        }
        for new in ["claude-opus-4-7", "claude-opus-4-8", "claude-opus-5-5", "claude-sonnet-5-5", "claude-fable-5-1"] {
            assert!(!anthropic_takes_budget(new), "{new}");
        }

        let anthropic = |model: &str, thinking: bool| {
            build_call(
                &Provider::resolve("anthropic", ""),
                "https://api.anthropic.com/v1",
                "k",
                model,
                &request(thinking),
            )
            .body
        };
        let on = anthropic("claude-opus-5-5", true);
        assert_eq!(on["thinking"]["type"], "adaptive");
        assert!(on["thinking"].get("budget_tokens").is_none());
        assert!(on.get("temperature").is_none());
        let off = anthropic("claude-opus-5-5", false);
        assert!(off.get("thinking").is_none());
        assert!(off.get("temperature").is_none());
        assert_eq!(off["output_config"]["effort"], "low");
        let legacy_off = anthropic("claude-sonnet-4-5", false);
        assert_eq!(legacy_off["temperature"], json!(0.7f32));
        assert!(legacy_off.get("output_config").is_none());
    }

    #[test]
    fn anthropic_and_gemini_use_native_shapes() {
        let a = build_call(
            &Provider::resolve("anthropic", ""),
            "https://api.anthropic.com/v1",
            "k",
            "claude-sonnet-4-5",
            &request(true),
        );
        assert_eq!(a.url, "https://api.anthropic.com/v1/messages");
        assert_eq!(a.body["system"], "sys");
        assert_eq!(a.body["thinking"]["budget_tokens"], 1024);
        assert!(a.body.get("temperature").is_none());
        assert!(a.headers.iter().any(|(k, v)| *k == "x-api-key" && v == "k"));

        let g = build_call(
            &Provider::resolve("gemini", ""),
            "https://generativelanguage.googleapis.com/v1beta",
            "k",
            "models/gemini-x",
            &request(true),
        );
        assert_eq!(
            g.url,
            "https://generativelanguage.googleapis.com/v1beta/models/gemini-x:generateContent"
        );
        assert_eq!(g.body["generationConfig"]["thinkingConfig"]["includeThoughts"], true);

        let gemini = |model: &str| {
            build_call(
                &Provider::resolve("gemini", ""),
                "https://generativelanguage.googleapis.com/v1beta",
                "k",
                model,
                &request(false),
            )
            .body["generationConfig"]
                .clone()
        };
        // Thinking off: switched off where possible, otherwise given headroom.
        let flash = gemini("gemini-2.5-flash");
        assert_eq!(flash["thinkingConfig"]["thinkingBudget"], 0);
        assert_eq!(flash["maxOutputTokens"], request(false).max_tokens);
        let pro = gemini("gemini-2.5-pro");
        assert!(pro.get("thinkingConfig").is_none());
        assert_eq!(pro["maxOutputTokens"], request(false).max_tokens + GEMINI_THINKING_HEADROOM);
    }

    #[test]
    fn parses_reasoning_per_protocol() {
        let openai = json!({ "choices": [{ "message": {
            "content": "answer", "reasoning_content": "because"
        }}]});
        assert_eq!(
            parse_reply(Protocol::OpenAi, &openai),
            AiReply { content: "answer".into(), reasoning: Some("because".into()) }
        );

        let inline = json!({ "choices": [{ "message": { "content": "<think>\nhmm\n</think>\n\nok" }}]});
        let reply = parse_reply(Protocol::OpenAi, &inline);
        assert_eq!(reply.content.trim(), "ok");
        assert_eq!(reply.reasoning.as_deref(), Some("hmm"));

        let anthropic = json!({ "content": [
            { "type": "thinking", "thinking": "plan" },
            { "type": "text", "text": "done" }
        ]});
        assert_eq!(
            parse_reply(Protocol::Anthropic, &anthropic),
            AiReply { content: "done".into(), reasoning: Some("plan".into()) }
        );

        let gemini = json!({ "candidates": [{ "content": { "parts": [
            { "text": "idea", "thought": true },
            { "text": "result" }
        ]}}]});
        assert_eq!(
            parse_reply(Protocol::Gemini, &gemini),
            AiReply { content: "result".into(), reasoning: Some("idea".into()) }
        );

        let plain = json!({ "candidates": [{ "content": { "parts": [{ "text": "only" }] }}]});
        assert_eq!(parse_reply(Protocol::Gemini, &plain).reasoning, None);
    }
}
