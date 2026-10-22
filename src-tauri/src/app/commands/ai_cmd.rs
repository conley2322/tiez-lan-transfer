use crate::database::DbState;
use crate::error;
use crate::error::{AppError, AppResult};
use crate::infrastructure::http_error::{body_for_log, summarize_http_error};
use crate::infrastructure::repository::clipboard_repo::ClipboardRepository;
use crate::infrastructure::repository::settings_repo::SettingsRepository;
use super::ai_provider::{build_call, parse_reply, AiReply, AiRequest, HttpCall, Provider};
use serde::Serialize;
use tauri::{command, AppHandle, Emitter, Manager};

/// reqwest's own message is only "error sending request"; the reason (refused,
/// timed out, certificate, DNS) is further down the source chain. The URL is
/// left out: it can carry an API key in the query.
fn network_error_text(e: reqwest::Error) -> String {
    let e = e.without_url();
    let mut parts = vec![e.to_string()];
    let mut source = std::error::Error::source(&e);
    while let Some(cause) = source {
        let text = cause.to_string();
        if !text.is_empty() && !parts.iter().any(|p| p.contains(&text)) {
            parts.push(text);
        }
        source = cause.source();
    }
    parts.join(": ")
}

const DEFAULT_BASE_URL: &str = "https://api.deepseek.com/v1";

fn clean_url(url: &str) -> String {
    url.trim().trim_end_matches('/').to_string()
}

async fn send_call(
    client: &reqwest::Client,
    call: &HttpCall,
) -> Result<reqwest::Response, reqwest::Error> {
    let mut request = client.post(&call.url).json(&call.body);
    for (name, value) in &call.headers {
        request = request.header(*name, value);
    }
    request.send().await
}

struct ProfileConfig {
    provider: Provider,
    api_key: String,
    base_url: String,
    model: String,
    enable_thinking: bool,
    reasoning_effort: String,
    thinking_budget: Option<i32>,
}

fn profile_config(profile: Option<&serde_json::Value>) -> ProfileConfig {
    let text = |field: &str| {
        profile
            .and_then(|p| p[field].as_str())
            .map(str::to_string)
            .unwrap_or_default()
    };
    // Only the no-profile default may fall back to DeepSeek. A profile with an
    // empty URL must not send its key (Claude, Gemini, custom…) to DeepSeek.
    let base_url = match text("baseUrl") {
        url if url.trim().is_empty() && profile.is_none() => DEFAULT_BASE_URL.to_string(),
        url => clean_url(&url),
    };
    let model = match (profile, text("model")) {
        (Some(_), m) if m.is_empty() => "deepseek-flash".to_string(),
        (_, m) => m,
    };
    let effort = text("reasoningEffort");
    ProfileConfig {
        provider: Provider::resolve(&text("provider"), &base_url),
        api_key: text("apiKey"),
        base_url,
        model,
        enable_thinking: profile
            .and_then(|p| p["enableThinking"].as_bool())
            .unwrap_or(false),
        reasoning_effort: if effort.is_empty() { "medium".to_string() } else { effort },
        thinking_budget: profile
            .and_then(|p| p["thinkingBudget"].as_i64())
            .and_then(|v| i32::try_from(v).ok()),
    }
}

impl ProfileConfig {
    /// Has what a request needs: an endpoint, plus a key unless the server is local.
    fn is_callable(&self) -> bool {
        !self.base_url.trim().is_empty()
            && (!self.provider.requires_key || !self.api_key.trim().is_empty())
    }
}

/// The profile a task runs on. The assigned one wins when it can be called;
/// otherwise the first callable profile in list order. The DeepSeek presets
/// ship without a key and are the default assignment, so a user who adds a
/// model of their own is not stuck on "API Key is not set" until they also
/// re-assign every task. With nothing callable, the assigned (or first)
/// profile is returned so the error names what is missing.
fn pick_profile<'a>(
    profiles: Option<&'a [serde_json::Value]>,
    assigned_id: &str,
) -> Option<&'a serde_json::Value> {
    let list = profiles.unwrap_or_default();
    let assigned = list
        .iter()
        .find(|p| p["id"].as_str() == Some(assigned_id))
        .or_else(|| list.first());
    if assigned.is_some_and(|p| profile_config(Some(p)).is_callable()) {
        return assigned;
    }
    list.iter()
        .find(|p| profile_config(Some(p)).is_callable())
        .or(assigned)
}

/// Characters of an entry the AI sees. Longer entries are cut, and the
/// result is then flagged so it can't replace the whole entry.
const AI_INPUT_LIMIT: usize = 2000;
/// Upper bound on a provider's response body; the timeout alone doesn't cap it.
const MAX_RESPONSE_BYTES: usize = 4 * 1024 * 1024;

/// A reply plus whether it was produced from a cut-down input.
#[derive(Serialize)]
pub struct AiResult {
    #[serde(flatten)]
    reply: AiReply,
    truncated: bool,
}

async fn read_body_capped(mut response: reqwest::Response) -> Result<String, reqwest::Error> {
    let mut body = Vec::new();
    while let Some(chunk) = response.chunk().await? {
        if body.len() + chunk.len() > MAX_RESPONSE_BYTES {
            break;
        }
        body.extend_from_slice(&chunk);
    }
    Ok(String::from_utf8_lossy(&body).into_owned())
}

/// Returns the text to send and whether it had to be cut.
fn sanitize_input(content: &str) -> AppResult<(String, bool)> {
    // "你是" ("you are") is left out: it is everyday Chinese, and blocked
    // ordinary entries as well as follow-ups quoting the AI's own question.
    let forbidden = ["忽略以上", "system prompt", "以上指令"];
    for word in forbidden {
        if content.contains(word) {
            return Err(AppError::Validation(
                "Input contains potential injection keywords".to_string(),
            ));
        }
    }
    let truncated = content.chars().count() > AI_INPUT_LIMIT;
    Ok((content.chars().take(AI_INPUT_LIMIT).collect(), truncated))
}

fn build_system_prompt(action_type: &str) -> String {
    // 1. 社交嘴替 (Mouthpiece) - 纯回复与防御
    let mouth_persona = "你不是聊天机器人，你是用户的社交嘴替与语言防御系统。\n\
        你的任务只有一个：替用户回复对方说的话。不要生成评价、摘要、文案或任何额外任务内容。\n\n\
        【核心定位】\n\
        - 外在：情绪稳定、语气自然、不咄咄逼人\n\
        - 内核：边界清晰、逻辑自洽、不吃亏\n\
        - 风格：简短有力，话少但句句有落点\n\n\
        【判断顺序（必须遵守）】\n\
        1. 是否在索取资源/情绪/责任\n\
        2. 是否在贬低、否定、试探或施压\n\
        3. 是否存在身份或权力不对等\n\
        4. 是否需要反击，还是只需止损封口\n\n\
        【输出策略】\n\
        - 优先一到两句话解决，避免解释和说教\n\
        - 不讨好、不示弱、不留口子\n\
        - 对方语气越重，你的语气越稳\n\
        - 若信息不足，可用一句话澄清关键点\n\n\
        【语言规则】\n\
        - 日常口语但不低级\n\
        - 不使用 Emoji\n\
        - 禁止在输出前后加引号或括号\n\
        - 不输出任何分析、思考、理由或过程\n\n\
        【目标】\n\
        - 面对攻击或压力：让对方自然停止继续进攻。";

    // 2. 任务解决助手 (Task Solver) - 通用任务执行
    let task_persona =
        "你是一个专注于“任务解决”的智能助手。你的核心目标是高效、精准、专业地完成用户的需求。\n\n\
        【你的核心原则】\n\
        1. **结果导向**：直接提供用户需要的内容，不要废话，不要过度寒暄。\n\
        2. **逻辑严密**：确保生成的内容结构清晰、论证有力。\n\
        3. **专业可靠**：保持客观、专业的语气，除非用户指定了特定风格。\n\
        4. **缺信息先问**：只有缺少关键信息、无法完成任务时，才只输出一行 [[QUESTION:你的问题]]，不要输出其他内容。";

    // 3. 翻译专家 (Translator)
    let translator_persona = "你现在的具体任务是【顶级同传】：请将内容翻译成目标语言，确保译文地道且富有生活化语气，严禁机翻感。只输出译文，不要哪怕一句多余的解释。";

    match action_type {
        "mouthpiece" => mouth_persona.to_string(),
        "task" => task_persona.to_string(),
        "translate" => translator_persona.to_string(),
        _ => mouth_persona.to_string(),
    }
}

fn build_user_prompt(action_type: &str, content: &str, target_lang: &str) -> String {
    let target_lang_name = match target_lang {
        "en" => "English",
        "ja" => "Japanese",
        "de" => "German",
        "fr" => "French",
        _ => "Chinese",
    };

    match action_type {
        "task" => content.to_string(),
        "mouthpiece" => format!("这是对方的内容或消息：\n\n“{}”\n\n请只针对这句话进行回复（嘴替模式），不要做其他任务。", content),
        "translate" => format!("请将以下内容翻译为 {}：\n\n{}", target_lang_name, content),
        _ => content.to_string(),
    }
}

#[command]
pub async fn check_ai_connectivity(
    base_url: String,
    api_key: String,
    model: String,
    provider: Option<String>,
) -> AppResult<String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .map_err(|e| format!("Client creation failed: {}", e))?;
    let cleaned_base = clean_url(&base_url);
    let provider = Provider::resolve(provider.as_deref().unwrap_or(""), &cleaned_base);
    // Same rule as call_ai, so a passing check means calls work too.
    if provider.requires_key && api_key.trim().is_empty() {
        return Err(AppError::Validation(
            "AI API Key is not set. Add one in AI Assistant Settings.".to_string(),
        ));
    }

    // Perform a real minimal handshake. Reasoning models may spend a few
    // tokens before answering, so a single token is not always accepted.
    let call = build_call(
        &provider,
        &cleaned_base,
        &api_key,
        &model,
        &AiRequest {
            system: "Reply with OK.",
            user: "ping",
            max_tokens: 16,
            temperature: 0.0,
            presence_penalty: 0.0,
            thinking: false,
            thinking_budget: 0,
            reasoning_effort: "low",
        },
    );
    let api_url = call.url.clone();

    let response = send_call(&client, &call)
        .await
        .map_err(|e| {
            error!("[AI] connectivity request to {} failed: {}", api_url, e);
            AppError::Network(network_error_text(e))
        })?;

    let status = response.status();
    if status.is_success() {
        Ok("success".to_string())
    } else {
        let err_text = response.text().await.unwrap_or_default();
        error!(
            "[AI] connectivity check {} returned {}: {}",
            api_url,
            status,
            body_for_log(&err_text)
        );
        Err(AppError::Network(summarize_http_error(status, &err_text)))
    }
}

#[command]
pub async fn call_ai(
    app_handle: AppHandle,
    id: i64,
    mut content: String,
    action_type: String, // "task", "mouthpiece", "translate"
    use_provided_content: Option<bool>,
) -> AppResult<AiResult> {
    let state = app_handle.state::<DbState>();

    // 0. Resolve full content if ID is provided and content is placeholder/truncated
    // Follow-up prompts already include the full original and the user's
    // answer. Resolve ordinary requests from storage, but keep that prompt.
    if id != 0 && !use_provided_content.unwrap_or(false) {
        if id > 0 {
            // Fetch from Database
            if let Ok(Some(full_content)) = state.repo.get_entry_content(id) {
                content = full_content;
            }
        } else {
            // Fetch from Session
            let session = app_handle.state::<crate::app_state::SessionHistory>();
            let session_items = session.0.lock().unwrap();
            if let Some(item) = session_items.iter().find(|i| i.id == id) {
                content = item.content.clone();
            }
        }
    }

    let (content, truncated) = sanitize_input(&content)?;

    let (config, target_lang, thinking_budget) = {
        // 1. Get functional assignment
        let assigned_profile_key = format!("ai_assigned_profile_{}", action_type);
        let assigned_id = state
            .settings_repo
            .get(&assigned_profile_key)
            .unwrap_or(None)
            .unwrap_or_else(|| "default".to_string());

        // 2. Get Profiles Library
        let profiles_json = state
            .settings_repo
            .get("ai_profiles")
            .unwrap_or(None)
            .unwrap_or_else(|| "[]".to_string());

        let profiles: serde_json::Value =
            serde_json::from_str(&profiles_json).unwrap_or(serde_json::json!([]));

        // 3. Find assigned profile, falling back to the first callable one
        let profile = pick_profile(profiles.as_array().map(Vec::as_slice), &assigned_id);
        let config = profile_config(profile);

        let lang = state
            .settings_repo
            .get("ai_target_lang")
            .unwrap_or(None)
            .unwrap_or_else(|| "zh".to_string());
        // Profiles saved before per-profile budgets fall back to the old global setting.
        let thinking_budget = config
            .thinking_budget
            .or_else(|| {
                state
                    .settings_repo
                    .get("ai_thinking_budget")
                    .unwrap_or(None)
                    .and_then(|v| v.parse::<i32>().ok())
            })
            .map(|v| v.clamp(1024, 32_000)) // APIs require at least 1024
            .unwrap_or(1024);

        (config, lang, thinking_budget)
    };

    if config.base_url.trim().is_empty() {
        return Err(AppError::Validation(
            "AI API URL is not set. Add one in AI Assistant Settings.".to_string(),
        ));
    }
    // Only the user's own key is ever sent; there is no built-in fallback key.
    if config.provider.requires_key && config.api_key.trim().is_empty() {
        return Err(AppError::Validation(
            "AI API Key is not set. Add one in AI Assistant Settings.".to_string(),
        ));
    }

    let effective_thinking = config.enable_thinking;

    // Handle Auto Detect for Translation
    let effective_target_lang = if action_type == "translate" && target_lang == "auto_zh_en" {
        // Simple heuristic: check if content contains any CJK unified ideographs
        let has_chinese = content.chars().any(
            |c| {
                (c >= '\u{4E00}' && c <= '\u{9FFF}') || // CJK Unified Ideographs
            (c >= '\u{3400}' && c <= '\u{4DBF}')
            }, // CJK Unified Ideographs Extension A
        );
        if has_chinese {
            "en".to_string()
        } else {
            "zh".to_string()
        }
    } else {
        target_lang
    };

    let system_prompt = build_system_prompt(&action_type);
    let user_prompt = build_user_prompt(&action_type, &content, &effective_target_lang);

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .map_err(|e| format!("Client creation failed: {}", e))?;

    // Ensure max_tokens is greater than thinking_budget when thinking is enabled
    // This prevents the thinking process from consuming all tokens, leaving no room for actual output
    // API requires: max_tokens > thinking_budget
    let base_max_tokens = 2000;
    let final_max_tokens = if effective_thinking {
        // Add large buffer: max_tokens should be at least thinking_budget + 1500 (for actual response)
        std::cmp::max(base_max_tokens, thinking_budget + 1500)
    } else {
        base_max_tokens
    };

    // Dynamic presence penalty
    let presence_penalty = if action_type == "mouthpiece" {
        0.6
    } else {
        0.0 // Task/Translate usually needs accuracy, not variety
    };

    let call = build_call(
        &config.provider,
        &config.base_url,
        &config.api_key,
        &config.model,
        &AiRequest {
            system: &system_prompt,
            user: &user_prompt,
            max_tokens: final_max_tokens,
            temperature: 0.7,
            presence_penalty,
            thinking: effective_thinking,
            thinking_budget,
            reasoning_effort: &config.reasoning_effort,
        },
    );
    let api_url = call.url.clone();

    let response = send_call(&client, &call)
        .await
        .map_err(|e| {
            error!("[AI] request to {} failed: {}", api_url, e);
            AppError::Network(network_error_text(e))
        })?;

    let status = response.status();
    if !status.is_success() {
        let err_text = response.text().await.unwrap_or_default();
        error!(
            "[AI] API {} returned {}: {}",
            api_url,
            status,
            body_for_log(&err_text)
        );
        return Err(AppError::Network(summarize_http_error(status, &err_text)));
    }

    let response_text = read_body_capped(response).await.map_err(|e| {
        error!("[AI] failed to read response body from {}: {}", api_url, e);
        AppError::Network(network_error_text(e))
    })?;

    let completion: serde_json::Value = serde_json::from_str(&response_text).map_err(|e| {
        // A 2xx body carries the user's own text; log its size, not its content.
        error!(
            "[AI] failed to parse response from {}: {} ({} bytes)",
            api_url,
            e,
            response_text.len()
        );
        AppError::Network("unexpected response from AI provider".to_string())
    })?;

    let AiReply {
        content: raw_response,
        reasoning,
    } = parse_reply(config.provider.protocol, &completion);

    let mut ai_response = raw_response.trim().to_string();

    // Safety check for empty results early
    if ai_response.is_empty() {
        error!(
            "[AI] empty content from {} ({} bytes)",
            api_url,
            response_text.len()
        );
        return Err(AppError::Internal(
            "AI returned an empty response".to_string(),
        ));
    }

    // UTF-8 Safe Quote Stripping - ONLY for Mouthpiece
    if action_type == "mouthpiece" {
        let has_quotes = {
            let chars: Vec<char> = ai_response.chars().collect();
            if chars.len() >= 2 {
                let first = chars[0];
                let last = chars[chars.len() - 1];
                (first == '"' && last == '"')
                    || (first == '\'' && last == '\'')
                    || (first == '“' && last == '”')
            } else {
                false
            }
        };

        if has_quotes {
            let mut chars = ai_response.chars();
            chars.next(); // remove first
            chars.next_back(); // remove last
            ai_response = chars.collect::<String>().trim().to_string();
        }
    }

    if ai_response.is_empty() {
        error!(
            "[AI] response became empty after cleaning; original: {}",
            body_for_log(&raw_response)
        );
        return Err(AppError::Internal(
            "AI response became empty after cleaning".to_string(),
        ));
    }

    // The entry stays untouched until the user confirms with apply_ai_result.
    Ok(AiResult {
        reply: AiReply {
            content: ai_response,
            reasoning,
        },
        truncated,
    })
}

/// Replaces an entry's content with a confirmed AI result.
// async: run off the main thread (DB lock + re-encryption).
#[command(async)]
pub fn apply_ai_result(app_handle: AppHandle, id: i64, content: String) -> AppResult<()> {
    let content = content.trim().to_string();
    if content.is_empty() {
        return Err(AppError::Validation("AI result is empty".to_string()));
    }
    let state = app_handle.state::<DbState>();
    let persistent = state
        .settings_repo
        .get("app.persistent")
        .unwrap_or(None)
        .unwrap_or_else(|| "true".to_string())
        == "true";
    let preview = if content.chars().count() > 100 {
        let p: String = content.chars().take(100).collect();
        format!("{}...", p.replace('\n', " "))
    } else {
        content.replace('\n', " ")
    };

    // 1. Update Session History (Memory State) - ALWAYS do this
    use crate::app_state::SessionHistory;
    let mut updated_item_found = false;
    if let Some(session) = app_handle.try_state::<SessionHistory>() {
        let mut history = session.0.lock().unwrap();
        if let Some(item) = history.iter_mut().find(|i| i.id == id) {
            item.content = content.clone();
            item.preview = preview.clone();
            if item.content_type != "image" {
                item.content_type = "text".to_string();
                item.html_content = None;
            }
            updated_item_found = true;

            // For session items (id < 0) or when persistence is off, emit the updated item directly from memory
            if !persistent || id < 0 {
                let _ = app_handle.emit("clipboard-updated", item.clone());
            }
        }
    }

    // 2. Update Database (if item is persistent and persistence is enabled)
    if persistent && id > 0 {
        state
            .repo
            .update_entry_content(id, &content, &preview)
            .map_err(|e| AppError::Internal(format!("Database update failed: {}", e)))?;
        // Emit update event from database to ensure consistency for persistent items
        if let Ok(Some(updated_entry)) = state.repo.get_entry_by_id(id) {
            let _ = app_handle.emit("clipboard-updated", updated_entry);
        }
        crate::services::cloud_sync::request_cloud_sync(app_handle.clone());
    } else if !updated_item_found {
        return Err(AppError::Validation(format!("Entry {} not found", id)));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{network_error_text, pick_profile};
    use serde_json::json;

    fn picked(profiles: &serde_json::Value, assigned: &str) -> Option<String> {
        pick_profile(profiles.as_array().map(Vec::as_slice), assigned)
            .and_then(|p| p["id"].as_str().map(str::to_string))
    }

    fn presets_plus(extra: serde_json::Value) -> serde_json::Value {
        let mut list = vec![
            json!({"id": "ds_v4_pro_v1", "provider": "deepseek", "baseUrl": "https://api.deepseek.com/v1", "apiKey": "", "model": "deepseek-v4-pro"}),
            json!({"id": "ds_flash_v1", "provider": "deepseek", "baseUrl": "https://api.deepseek.com/v1", "apiKey": "", "model": "deepseek-flash"}),
        ];
        if let Some(extra) = extra.as_array() {
            list.extend(extra.iter().cloned());
        }
        serde_json::Value::Array(list)
    }

    #[test]
    fn keyless_assignment_falls_back_to_a_model_with_a_key() {
        let own = json!([{"id": "1", "provider": "openai", "baseUrl": "https://api.openai.com/v1", "apiKey": "sk-x", "model": "gpt"}]);
        let profiles = presets_plus(own);
        assert_eq!(picked(&profiles, "ds_flash_v1").as_deref(), Some("1"));
        // A deleted assignment ("default") also lands on the callable model.
        assert_eq!(picked(&profiles, "default").as_deref(), Some("1"));
    }

    #[test]
    fn callable_assignment_is_kept() {
        let mut profiles = presets_plus(json!([
            {"id": "1", "provider": "openai", "baseUrl": "https://api.openai.com/v1", "apiKey": "sk-x", "model": "gpt"}
        ]));
        profiles[1]["apiKey"] = json!("sk-ds");
        assert_eq!(picked(&profiles, "ds_flash_v1").as_deref(), Some("ds_flash_v1"));
        assert_eq!(picked(&profiles, "1").as_deref(), Some("1"));
    }

    #[test]
    fn local_models_need_no_key() {
        let profiles = presets_plus(json!([
            {"id": "local", "provider": "ollama", "baseUrl": "http://localhost:11434/v1", "apiKey": "", "model": "qwen3:8b"}
        ]));
        assert_eq!(picked(&profiles, "ds_v4_pro_v1").as_deref(), Some("local"));
    }

    #[test]
    fn nothing_callable_keeps_the_assignment_for_the_error() {
        let profiles = presets_plus(json!([]));
        assert_eq!(picked(&profiles, "ds_flash_v1").as_deref(), Some("ds_flash_v1"));
        assert_eq!(picked(&json!([]), "default"), None);
    }

    #[test]
    fn network_errors_say_why_the_request_failed() {
        let _ = rustls::crypto::ring::default_provider().install_default();
        // Bind and drop a listener: the port is then free and refuses connections.
        let port = std::net::TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap().port();
        let url = format!("http://127.0.0.1:{port}/v1/chat/completions?key=secret");
        let err = tauri::async_runtime::block_on(async {
            reqwest::Client::new().get(&url).send().await.unwrap_err()
        });
        let text = network_error_text(err);
        assert!(text.starts_with("error sending request"), "{text}");
        assert!(text.len() > "error sending request".len() + 2, "no cause in: {text}");
        assert!(!text.contains("secret"), "URL leaked into: {text}");
    }
}
