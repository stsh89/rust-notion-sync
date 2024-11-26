use eyre::Report;
use std::time::Duration;
use ureq::Response;

#[derive(Debug, thiserror::Error)]
#[error("Notion API request failure")]
pub struct Error {
    kind: ErrorKind,
    source: Report,
}

#[derive(Debug)]
enum ErrorKind {
    Authorization,
    BadRequest,
    Communication,
    RateLimit(Duration),
    Unexpected,
}

impl Error {
    pub fn is_authorization(&self) -> bool {
        matches!(self.kind, ErrorKind::Authorization)
    }

    pub fn is_bad_request(&self) -> bool {
        matches!(self.kind, ErrorKind::BadRequest)
    }

    pub fn is_communication(&self) -> bool {
        matches!(self.kind, ErrorKind::Communication)
    }

    pub fn is_rate_limit(&self) -> bool {
        matches!(self.kind, ErrorKind::RateLimit(_))
    }

    pub fn is_unexptected_status(&self) -> bool {
        matches!(self.kind, ErrorKind::Unexpected)
    }

    pub fn retry_after(&self) -> Option<Duration> {
        match self.kind {
            ErrorKind::RateLimit(duration) => Some(duration),
            _ => None,
        }
    }
}

// Integrations should accommodate variable rate limits by handling HTTP 429 responses
// and respecting the Retry-After response header value,
// which is set as an integer number of seconds (in decimal).
// See more for details https://developers.notion.com/reference/request-limits
impl From<ureq::Error> for Error {
    fn from(err: ureq::Error) -> Self {
        let kind = match &err {
            ureq::Error::Transport(_) => ErrorKind::Communication,
            ureq::Error::Status(400, _) => ErrorKind::BadRequest,
            ureq::Error::Status(401, _) => ErrorKind::Authorization,
            ureq::Error::Status(429, response) => rate_limit_error_kind(response),
            ureq::Error::Status(_, _) => ErrorKind::Unexpected,
        };

        Self {
            kind,
            source: Report::new(err),
        }
    }
}

fn rate_limit_error_kind(response: &Response) -> ErrorKind {
    let retry_after = response.header("Retry-After").unwrap_or_else(|| {
        tracing::warn!("Notion API response returned 429 status code without Retry-After header");

        "1.0"
    });

    let seconds = retry_after.parse::<f64>().unwrap_or_else(|_value| {
        tracing::warn!(
            "Notion API response returned 429 status code with invalid Retry-After header: {}",
            retry_after
        );

        1.0
    });

    let duration = Duration::from_secs_f64(seconds);
    tracing::warn!("Notion API request rate limited for {:?}", duration);

    ErrorKind::RateLimit(duration)
}
