use serde_json::Value as Json;
use std::num::NonZeroU32;

pub struct CreateDatabaseEntryParameters<'a> {
    pub database_id: &'a str,
    pub properties: Json,
}

pub struct ClientParameters {
    pub api_key: String,
    pub base_url_override: Option<String>,
}

pub struct QueryDatabaseParameters<'a> {
    pub database_id: &'a str,
    pub filter: Option<Json>,
    pub page_size: Option<NonZeroU32>,
    pub start_cursor: Option<&'a str>,
}

#[derive(Default)]
pub struct RetryParameters<F> {
    pub custom_sleep: Option<F>,
}

pub struct UpdateDatabaseEntryParameters<'a> {
    pub entry_id: &'a str,
    pub properties: Json,
}
