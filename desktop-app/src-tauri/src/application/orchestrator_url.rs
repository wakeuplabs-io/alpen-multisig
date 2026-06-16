//! Orchestrator backend URL policy — shared by auth and proposal IPC paths.

const INSECURE_HTTP_ERROR: &str =
    "orchestrator base_url must use https:// (http://localhost or 127.0.0.1 allowed for local dev only)";

pub fn validate_orchestrator_base_url(base_url: &str) -> Result<(), String> {
    let trimmed = base_url.trim();
    if trimmed.starts_with("https://") {
        return Ok(());
    }
    if trimmed.starts_with("http://localhost")
        || trimmed.starts_with("http://127.0.0.1")
        || trimmed.starts_with("http://[::1]")
    {
        return Ok(());
    }
    let allow_insecure = std::env::var("ALLOW_INSECURE_HTTP_URL")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);
    if allow_insecure {
        return Ok(());
    }
    Err(INSECURE_HTTP_ERROR.to_string())
}

#[cfg(test)]
mod tests {
    use super::validate_orchestrator_base_url;

    #[test]
    fn rejects_plain_http_remote() {
        assert!(validate_orchestrator_base_url("http://evil.example/api/v1").is_err());
    }

    #[test]
    fn allows_https() {
        assert!(validate_orchestrator_base_url("https://orchestrator.example/api/v1").is_ok());
    }

    #[test]
    fn allows_localhost_http() {
        assert!(validate_orchestrator_base_url("http://127.0.0.1:3000/api/v1").is_ok());
    }
}
