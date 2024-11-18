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
