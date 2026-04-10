use thiserror::Error;

/// All errors that can occur when calling the Black Duck API.
#[derive(Debug, Error)]
pub enum ApiError {
    /// HTTP transport error (reqwest)
    #[error("HTTP error: {0}")]
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
