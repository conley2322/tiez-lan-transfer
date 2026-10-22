fn non_empty_runtime_env(key: &str) -> Option<String> {
    std::env::var(key).ok().and_then(|value| {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}

pub fn announcement_ping_url() -> String {
    non_empty_runtime_env("TIEZ_ANNOUNCEMENT_PING_URL")
        .or_else(|| option_env!("TIEZ_ANNOUNCEMENT_PING_URL").map(|value| value.to_string()))
        .unwrap_or_else(|| "https://tiez.name666.top/api/v1/ping".to_string())
}

pub fn theme_store_base_url() -> String {
    non_empty_runtime_env("TIEZ_THEME_STORE_BASE_URL")
        .or_else(|| option_env!("TIEZ_THEME_STORE_BASE_URL").map(|value| value.to_string()))
        .unwrap_or_else(|| "https://tiez.name666.top".to_string())
        .trim_end_matches('/')
        .to_string()
}

pub fn default_theme_id() -> &'static str {
    #[cfg(target_os = "macos")]
    {
        "macos"
    }
    #[cfg(not(target_os = "macos"))]
    {
        "mica"
    }
}
