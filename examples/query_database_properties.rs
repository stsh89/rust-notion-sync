use anyhow::Result;
use clap::Parser;
use rusty_notion::api::{self, Client, ClientParameters};
use serde_json::Value as Json;

#[derive(Parser)]
struct Cli {
    #[arg(long)]
    api_key: String,

    #[arg(long)]
    database_id: String,
}

fn main() -> Result<()> {
    let Cli {
        api_key,
        database_id,
    } = Cli::parse();

    let client = Client::new(ClientParameters {
        api_key,
        base_url_override: None,
    });

    let response = api::query_database_properties(&client, &database_id)?;

    println!("StatusCode : {}", response.status());
    println!("Content    : {}", response.into_json::<Json>()?);

    Ok(())
}
