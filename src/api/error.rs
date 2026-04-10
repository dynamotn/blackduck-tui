use thiserror::Error;

/// All errors that can occur when calling the Black Duck API.
#[derive(Debug, Error)]
pub enum ApiError {
    /// HTTP transport error (reqwest) — re-formatted into a concise message.
    #[error("{}", friendly_http_error(.0))]
    Http(#[from] reqwest::Error),

    /// The server returned a non-2xx status code
    #[error("{context} ({status}): {body}")]
    StatusCode {
        context: &'static str,
        status: reqwest::StatusCode,
        body: String,
    },

    /// The client has not been authenticated yet
    #[error("Client is not authenticated; call authenticate() first")]
    NotAuthenticated,
}

/// Produce a concise, user-readable description of a `reqwest::Error`.
fn friendly_http_error(e: &reqwest::Error) -> String {
    if e.is_connect() {
        if let Some(url) = e.url() {
            return format!(
                "Cannot connect to {} — is the server reachable?",
                url.host_str().unwrap_or(url.as_str())
            );
        }
        return "Cannot connect to server — is the server reachable?".to_string();
    }
    if e.is_timeout() {
        return "Request timed out — check network connectivity".to_string();
    }
    if e.is_builder() {
        return format!("Invalid URL or client configuration: {e}");
    }
    // reqwest wraps the hyper/h2 chain; look for recognisable sub-strings
    let msg = e.to_string();
    if msg.contains("dns error") || msg.contains("resolve host") || msg.contains("No such host") {
        let host = e
            .url()
            .and_then(|u| u.host_str().map(ToString::to_string))
            .unwrap_or_else(|| "host".to_string());
        return format!("DNS lookup failed for '{host}' — check the server URL and network");
    }
    if msg.contains("certificate")
        || msg.contains("tls")
        || msg.contains("ssl")
        || msg.contains("TLS")
    {
        return format!(
            "TLS/certificate error — try setting accept_invalid_certs = true in config: {e}"
        );
    }
    // Fallback: strip the verbose reqwest prefix
    format!("Network error: {e}")
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn not_authenticated_display() {
        let e = ApiError::NotAuthenticated;
        assert_eq!(
            e.to_string(),
            "Client is not authenticated; call authenticate() first"
        );
    }

    #[test]
    fn status_code_display() {
        let e = ApiError::StatusCode {
            context: "get projects",
            status: reqwest::StatusCode::UNAUTHORIZED,
            body: "Unauthorized".to_string(),
        };
        assert_eq!(
            e.to_string(),
            "get projects (401 Unauthorized): Unauthorized"
        );
    }

    #[test]
    fn status_code_404_display() {
        let e = ApiError::StatusCode {
            context: "get components",
            status: reqwest::StatusCode::NOT_FOUND,
            body: "not found".to_string(),
        };
        assert!(e.to_string().contains("404"));
        assert!(e.to_string().contains("get components"));
    }
}
