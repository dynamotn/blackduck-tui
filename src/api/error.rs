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
