mod failure;
mod headers;
mod parameters;

use headers::{SetAuthorizationHeader, SetDefaultHeaders};
use std::{num::NonZeroU32, time::Duration};
use ureq::{Agent, AgentBuilder, Response};

pub use failure::*;
pub use parameters::*;

pub type Result<T> = std::result::Result<T, Error>;

pub struct Client {
    inner: Agent,
    base_url: String,
    api_key: String,
}

impl Client {
    pub fn base_url(self, base_url: String) -> Self {
        Self { base_url, ..self }
    }

    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            inner: AgentBuilder::new().build(),
            base_url: "https://api.notion.com/v1".to_string(),
        }
    }
}

pub fn create_database_entry(
    client: &Client,
    parameters: CreateDatabaseEntryParameters,
) -> Result<Response> {
    let CreateDatabaseEntryParameters {
        database_id,
        properties,
    } = parameters;

    let path = format!("{}/pages", &client.base_url);

    let body = serde_json::json!({
        "parent": { "database_id": database_id },
        "properties": properties,
    });

    let response = client
        .inner
        .post(&path)
        .set_default_headers()
        .set_authorization_header(&client.api_key)
        .send_json(body)?;

    Ok(response)
}

pub fn query_database_properties(client: &Client, database_id: &str) -> Result<Response> {
    let path = format!("{}/databases/{}", &client.base_url, database_id);

    let response = client
        .inner
        .get(&path)
        .set_default_headers()
        .set_authorization_header(&client.api_key)
        .call()?;

    Ok(response)
}

pub fn query_database(client: &Client, parameters: QueryDatabaseParameters) -> Result<Response> {
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

    let response = client
        .inner
        .post(&path)
        .set_default_headers()
        .set_authorization_header(&client.api_key)
        .send_json(body)?;

    Ok(response)
}

pub fn send_with_retries(
    f: impl Fn() -> Result<Response>,
    sleep: impl Fn(Duration),
) -> Result<Response> {
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

        let err = result.unwrap_err();

        if !err.is_rate_limit() {
            tracing::warn!("Not retryable Notion API request error: {}", err);

            return Err(err);
        }

        if let Some(duration) = err.retry_after() {
            tracing::warn!(
                "Sleeping for {:?} before retrying Notion API request",
                duration
            );

            sleep(duration);
        }
    }
}

pub fn update_database_entry(
    client: &Client,
    parameters: UpdateDatabaseEntryParameters,
) -> Result<Response> {
    let UpdateDatabaseEntryParameters {
        entry_id,
        properties,
    } = parameters;

    let path = format!("{}/pages/{}", &client.base_url, entry_id);
    let body = serde_json::json!({"properties": properties});

    let response = client
        .inner
        .patch(&path)
        .set_default_headers()
        .set_authorization_header(&client.api_key)
        .send_json(body)?;

    Ok(response)
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU8, Ordering};

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

        let client = Client::new("test_api_key".to_string()).base_url(base_url);

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

        let client = Client::new("test_api_key".to_string()).base_url(base_url);

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

        let client = Client::new("test_api_key".to_string()).base_url(base_url);

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
    fn test_send_with_reries_returns_status_200() -> Result<()> {
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

        let client = Client::new("test_api_key".to_string()).base_url(base_url);

        let sleep_count = AtomicU8::new(0);

        let result = send_with_retries(
            || {
                query_database(
                    &client,
                    QueryDatabaseParameters {
                        database_id,
                        page_size: None,
                        start_cursor: None,
                        filter: None,
                    },
                )
            },
            |_duration| {
                sleep_count.fetch_add(1, Ordering::SeqCst);
            },
        );

        mock.assert();
        assert_eq!(result?.status(), 200);
        assert_eq!(sleep_count.load(Ordering::SeqCst), 0);

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

        let client = Client::new("test_api_key".to_string()).base_url(base_url);

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
