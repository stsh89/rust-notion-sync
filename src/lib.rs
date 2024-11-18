mod failure;
mod headers;
mod parameters;

use headers::{SetAuthorizationHeader, SetDefaultHeaders};
use std::{num::NonZeroU32, thread, time::Duration};
use ureq::{Agent, AgentBuilder, Response};

pub use failure::Error as NotionApiClientError;
pub use parameters::*;

pub type NotionApiClientResult<T> = std::result::Result<T, NotionApiClientError>;

pub struct NotionApiClient {
    inner: Agent,
    base_url: String,
    api_key: String,
}

impl NotionApiClient {
    pub fn new(parameters: NotionApiClientParameters) -> Self {
        let NotionApiClientParameters {
            api_key,
            base_url_override,
        } = parameters;

        let inner = AgentBuilder::new().build();
        let base_url = base_url_override.unwrap_or_else(|| "https://api.notion.com/v1".to_string());

        Self {
            api_key,
            inner,
            base_url,
        }
    }
}

pub fn create_database_entry(
    client: &NotionApiClient,
    parameters: CreateDatabaseEntryParameters,
) -> NotionApiClientResult<Response> {
    let CreateDatabaseEntryParameters {
        database_id,
        properties,
    } = parameters;

    let path = format!("{}/pages", &client.base_url);

    let body = serde_json::json!({
        "parent": { "database_id": database_id },
        "properties": properties,
    });

    client
        .inner
        .post(&path)
        .set_default_headers()
        .set_authorization_header(&client.api_key)
        .send_json(body)
        .map_err(api_client_error)
}

pub fn query_database_properties(
    client: &NotionApiClient,
    database_id: &str,
) -> NotionApiClientResult<Response> {
    let path = format!("{}/databases/{}", &client.base_url, database_id);

    client
        .inner
        .get(&path)
        .set_default_headers()
        .set_authorization_header(&client.api_key)
        .call()
        .map_err(api_client_error)
}

pub fn query_database(
    client: &NotionApiClient,
    parameters: QueryDatabaseParameters,
) -> NotionApiClientResult<Response> {
    let QueryDatabaseParameters {
        database_id,
        start_cursor,
        page_size,
        filter,
    } = parameters;

    let page_size = page_size
        .unwrap_or(unsafe { NonZeroU32::new_unchecked(100) })
        .get();

    tracing::info!(
        message = "Query Notion database",
        database_id = database_id,
        page_size = page_size,
        start_cursor = start_cursor
    );

    let path = format!("{}/databases/{}/query", &client.base_url, database_id);
    let mut body = serde_json::json!({"page_size": page_size});

    if let Some(start_cursor) = start_cursor {
        body["start_cursor"] = start_cursor.into();
    }

    if let Some(filter) = filter {
        body["filter"] = filter;
    }

    client
        .inner
        .post(&path)
        .set_default_headers()
        .set_authorization_header(&client.api_key)
        .send_json(body)
        .map_err(api_client_error)
}

pub fn send_with_retries<F, S>(
    parameters: RetryParameters<S>,
    f: F,
) -> NotionApiClientResult<Response>
where
    F: Fn() -> NotionApiClientResult<Response>,
    S: Fn(Duration),
{
    let RetryParameters {
        custom_sleep: sleep_override,
    } = parameters;

    let max_retries = 3;
    let mut retries = 0;

    loop {
        let result = f();

        if result.is_ok() {
            return result;
        }

        if retries == max_retries {
            tracing::error!(
                "Stoping to retry Notion API request after {} retries",
                max_retries
            );

            return result;
        }

        retries += 1;

        match result.unwrap_err() {
            NotionApiClientError::RateLimit(duration) => {
                tracing::warn!(
                    "Sleeping for {:?} before retrying Notion API request",
                    duration
                );

                match &sleep_override {
                    Some(sleep) => sleep(duration),
                    None => thread::sleep(duration),
                };
            }
            err => {
                tracing::warn!("Not retryable Notion API request error: {}", err);

                return Err(err);
            }
        }
    }
}

pub fn update_database_entry(
    client: &NotionApiClient,
    parameters: UpdateDatabaseEntryParameters,
) -> NotionApiClientResult<Response> {
    let UpdateDatabaseEntryParameters {
        entry_id,
        properties,
    } = parameters;

    let path = format!("{}/pages/{}", &client.base_url, entry_id);
    let body = serde_json::json!({"properties": properties});

    client
        .inner
        .patch(&path)
        .set_default_headers()
        .set_authorization_header(&client.api_key)
        .send_json(body)
        .map_err(api_client_error)
}

// Integrations should accommodate variable rate limits by handling HTTP 429 responses
// and respecting the Retry-After response header value,
// which is set as an integer number of seconds (in decimal).
// See more for details https://developers.notion.com/reference/request-limits
fn api_client_error(err: ureq::Error) -> NotionApiClientError {
    match err {
        ureq::Error::Transport(err) => NotionApiClientError::Transport(err.to_string()),
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

            NotionApiClientError::RateLimit(duration)
        }
        ureq::Error::Status(code, _) => NotionApiClientError::Status(code),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;
    use httpmock::{
        Method::{GET, PATCH, POST},
        MockServer,
    };
    use serde_json::json;

    #[test]
    fn test_create_database_entry_returns_status_200() -> Result<()> {
        let mock_notion_server = MockServer::start();
        let base_url = mock_notion_server.base_url();
        let database_id = "test_database_id";
        let properties = json!({
            "Name": {"title": [{"text": {"content": "Tuscan Kale"}}]}
        });

        let mock = mock_notion_server.mock(|when, then| {
            when.path("/pages")
                .method(POST)
                .header("Authorization", "Bearer test_api_key")
                .header("Content-Type", "application/json")
                .header("Notion-Version", "2022-06-28")
                .json_body(json!({
                    "parent": {
                        "database_id": database_id
                    },
                    "properties": properties
                }));

            then.status(200);
        });

        let client = NotionApiClient::new(NotionApiClientParameters {
            base_url_override: Some(base_url),
            api_key: "test_api_key".to_string(),
        });

        let result = create_database_entry(
            &client,
            CreateDatabaseEntryParameters {
                database_id,
                properties,
            },
        );

        mock.assert();
        assert_eq!(result?.status(), 200);

        Ok(())
    }

    #[test]
    fn test_get_database_properties_returns_status_200() -> Result<()> {
        let mock_notion_server = MockServer::start();
        let base_url = mock_notion_server.base_url();
        let database_id = "test_database_id";

        let mock = mock_notion_server.mock(|when, then| {
            when.path("/databases/test_database_id")
                .method(GET)
                .header("Authorization", "Bearer test_api_key")
                .header("Content-Type", "application/json")
                .header("Notion-Version", "2022-06-28");

            then.status(200);
        });

        let client = NotionApiClient::new(NotionApiClientParameters {
            base_url_override: Some(base_url),
            api_key: "test_api_key".to_string(),
        });

        let result = query_database_properties(&client, database_id);

        mock.assert();
        assert_eq!(result?.status(), 200);

        Ok(())
    }

    #[test]
    fn test_query_database_returns_status_200() -> Result<()> {
        let mock_notion_server = MockServer::start();
        let base_url = mock_notion_server.base_url();
        let database_id = "test_database_id";

        let mock = mock_notion_server.mock(|when, then| {
            when.path("/databases/test_database_id/query")
                .method(POST)
                .header("Authorization", "Bearer test_api_key")
                .header("Content-Type", "application/json")
                .header("Notion-Version", "2022-06-28");

            then.status(200);
        });

        let client = NotionApiClient::new(NotionApiClientParameters {
            base_url_override: Some(base_url),
            api_key: "test_api_key".to_string(),
        });

        let result = query_database(
            &client,
            QueryDatabaseParameters {
                database_id,
                page_size: None,
                start_cursor: None,
                filter: None,
            },
        );

        mock.assert();
        assert_eq!(result?.status(), 200);

        Ok(())
    }

    #[test]
    fn test_update_database_entry_returns_status_200() -> Result<()> {
        let mock_notion_server = MockServer::start();
        let base_url = mock_notion_server.base_url();
        let entry_id = "test_entry_id";
        let properties = json!({
            "Name": {"title": [{"text": {"content": "Tuscan Kale"}}]}
        });

        let mock = mock_notion_server.mock(|when, then| {
            when.path("/pages/test_entry_id")
                .method(PATCH)
                .header("Authorization", "Bearer test_api_key")
                .header("Content-Type", "application/json")
                .header("Notion-Version", "2022-06-28")
                .json_body(json!({"properties": properties}));

            then.status(200);
        });

        let client = NotionApiClient::new(NotionApiClientParameters {
            base_url_override: Some(base_url),
            api_key: "test_api_key".to_string(),
        });

        let result = update_database_entry(
            &client,
            UpdateDatabaseEntryParameters {
                entry_id,
                properties,
            },
        );

        mock.assert();
        assert_eq!(result?.status(), 200);

        Ok(())
    }
}
