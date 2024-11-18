# Rust-notion-sync

Raw synchronous API client for Notion.

## List of supported APIs

* `create_database_entry` - [Create a page](https://developers.notion.com/reference/post-page)
* `query_database_properties` - [Retrieve a database](https://developers.notion.com/reference/retrieve-a-database)
* `query_database` - [Query a database](https://developers.notion.com/reference/post-database-query)
* `update_database_entry` - [Update page properties](https://developers.notion.com/reference/patch-page)

## Usage

API calls should be wrapped into `send_with_retries` function in order to handle API rate limits properly.

## Related links

* [Notion API reference](https://developers.notion.com/reference).
* [Notion Offical API client library for rust](https://github.com/jakeswenson/notion).
