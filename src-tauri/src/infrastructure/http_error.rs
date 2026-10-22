//! Helpers that keep raw HTTP response bodies out of user-facing errors.
//!
//! The full body belongs in tiez.log (via `body_for_log`); the error returned to
//! the frontend only gets a short summary (via `summarize_http_error`).

use reqwest::StatusCode;

/// Max characters of the server-provided message kept in a user-facing error.
const MAX_MESSAGE_CHARS: usize = 160;
/// Max bytes of a response body written to the log.
const MAX_LOG_BODY_BYTES: usize = 4096;

/// "HTTP 502 Bad Gateway", plus ": <message>" only when the body is JSON that
/// carries a short textual error (error.message / message / error / detail).
/// HTML or other non-JSON bodies are never included.
pub fn summarize_http_error(status: StatusCode, body: &str) -> String {
    let head = match status.canonical_reason() {
        Some(reason) => format!("HTTP {} {}", status.as_u16(), reason),
        None => format!("HTTP {}", status.as_u16()),
    };
    match extract_json_message(body) {
        Some(message) => format!("{}: {}", head, message),
        None => head,
    }
}

/// Body text safe to write to the log: capped at a few KB on a char boundary.
pub fn body_for_log(body: &str) -> String {
    if body.len() <= MAX_LOG_BODY_BYTES {
        return body.to_string();
    }
    let mut end = MAX_LOG_BODY_BYTES;
    while !body.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}…[truncated {} bytes]", &body[..end], body.len() - end)
}

fn extract_json_message(body: &str) -> Option<String> {
    let value: serde_json::Value = serde_json::from_str(body.trim()).ok()?;
    let candidates = [
        value.pointer("/error/message"),
        value.get("message"),
        value.get("error"),
        value.get("detail"),
    ];
    let message = candidates
        .into_iter()
        .flatten()
        .filter_map(|v| v.as_str())
        .map(normalize_message)
        .find(|m| !m.is_empty() && !looks_like_html(m))?;
    Some(truncate_chars(&message, MAX_MESSAGE_CHARS))
}

fn normalize_message(raw: &str) -> String {
    raw.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn looks_like_html(text: &str) -> bool {
    let bytes = text.as_bytes();
    bytes
        .windows(2)
        .any(|w| w[0] == b'<' && (w[1].is_ascii_alphabetic() || w[1] == b'/' || w[1] == b'!'))
        && text.contains('>')
}

fn truncate_chars(text: &str, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        return text.to_string();
    }
    let mut out: String = text.chars().take(max_chars.saturating_sub(1)).collect();
    out.push('…');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nested_error_message_is_used() {
        let body = r#"{"error":{"message":"Invalid API key","type":"auth"}}"#;
        assert_eq!(
            summarize_http_error(StatusCode::UNAUTHORIZED, body),
            "HTTP 401 Unauthorized: Invalid API key"
        );
    }

    #[test]
    fn plain_message_and_fallback_fields() {
        assert_eq!(
            summarize_http_error(StatusCode::BAD_REQUEST, r#"{"message":"model not found"}"#),
            "HTTP 400 Bad Request: model not found"
        );
        assert_eq!(
            summarize_http_error(StatusCode::FORBIDDEN, r#"{"error":"quota exceeded"}"#),
            "HTTP 403 Forbidden: quota exceeded"
        );
        assert_eq!(
            summarize_http_error(
                StatusCode::UNPROCESSABLE_ENTITY,
                r#"{"detail":"bad input"}"#
            ),
            "HTTP 422 Unprocessable Entity: bad input"
        );
        // Non-string detail (e.g. FastAPI validation arrays) is ignored.
        assert_eq!(
            summarize_http_error(
                StatusCode::UNPROCESSABLE_ENTITY,
                r#"{"detail":[{"loc":1}]}"#
            ),
            "HTTP 422 Unprocessable Entity"
        );
    }

    #[test]
    fn html_body_is_never_included() {
        let body = "<!DOCTYPE html><html><head><title>502 Bad Gateway</title></head>\
                    <body><h1>502 Bad Gateway</h1><hr>nginx</body></html>";
        assert_eq!(
            summarize_http_error(StatusCode::BAD_GATEWAY, body),
            "HTTP 502 Bad Gateway"
        );
        let json_with_html = r#"{"message":"<html><body>oops</body></html>"}"#;
        assert_eq!(
            summarize_http_error(StatusCode::BAD_GATEWAY, json_with_html),
            "HTTP 502 Bad Gateway"
        );
    }

    #[test]
    fn empty_body_and_unknown_status() {
        assert_eq!(
            summarize_http_error(StatusCode::SERVICE_UNAVAILABLE, ""),
            "HTTP 503 Service Unavailable"
        );
        let odd = StatusCode::from_u16(599).unwrap();
        assert_eq!(summarize_http_error(odd, "   "), "HTTP 599");
    }

    #[test]
    fn very_long_bodies_are_capped() {
        let long_text = "x".repeat(10_000);
        assert_eq!(
            summarize_http_error(StatusCode::INTERNAL_SERVER_ERROR, &long_text),
            "HTTP 500 Internal Server Error"
        );
        let long_json = format!(r#"{{"message":"{}"}}"#, "错".repeat(1_000));
        let summary = summarize_http_error(StatusCode::INTERNAL_SERVER_ERROR, &long_json);
        assert!(
            summary.chars().count() <= 200,
            "{}",
            summary.chars().count()
        );
        assert!(summary.ends_with('…'));

        let logged = body_for_log(&"错".repeat(5_000));
        assert!(logged.len() <= MAX_LOG_BODY_BYTES + 64);
        assert!(logged.contains("[truncated"));
    }
}
