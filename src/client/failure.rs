use std::time::Duration;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Notion API request failure. Please retry in {0:?}")]
    RateLimit(Duration),

    #[error("Notion API request failed with status code {0}")]
    Status(u16),

    #[error("Notion API request failure: {0}")]
    Transport(String),
}

// Integrations should accommodate variable rate limits by handling HTTP 429 responses
// and respecting the Retry-After response header value,
// which is set as an integer number of seconds (in decimal).
// See more for details https://developers.notion.com/reference/request-limits
impl From<ureq::Error> for Error {
    fn from(err: ureq::Error) -> Self {
        match err {
            ureq::Error::Transport(err) => Error::Transport(err.to_string()),
            ureq::Error::Status(429, response) => {
                let retry_after = response.header("Retry-After").unwrap_or_else(|| {
                    tracing::warn!(
                        "Notion API response returned 429 status code without Retry-After header"
                    );

                    "1.0"
                });

                let seconds = retry_after.parse::<f64>().unwrap_or_else (|_value| {
                    tracing::warn!("Notion API response returned 429 status code with invalid Retry-After header: {}", retry_after);

                    1.0
                });

                let duration = Duration::from_secs_f64(seconds);
                tracing::warn!("Notion API request rate limited for {:?}", duration);

                Error::RateLimit(duration)
            }
            ureq::Error::Status(code, _) => Error::Status(code),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rate_limit_error_message() {
        let err = Error::RateLimit(Duration::from_secs_f64(0.23));

        assert_eq!(
            err.to_string(),
            "Notion API request failure. Please retry in 230ms"
        )
    }

    #[test]
    fn test_status_code_error_message() {
        let err = Error::Status(404);

        assert_eq!(
            err.to_string(),
            "Notion API request failed with status code 404"
        )
    }

    #[test]
    fn test_transport_error_message() {
        let err = Error::Transport("Cannot resolve the target name.".to_string());

        assert_eq!(
            err.to_string(),
            "Notion API request failure: Cannot resolve the target name."
        );
    }
}
