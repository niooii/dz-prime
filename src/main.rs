use std::{env, sync::Arc};
mod model;
mod bot;
mod scheduler;
mod jobs;
mod database;
mod time_parse;
use bot::DZBot;
use database::Database;
use serenity::prelude::*;
use anyhow::Result;


#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().expect("Expected a .env file");
    let token = env::var("DISCORD_TOKEN").expect("Expected DISCORD_TOKEN in the environment");
    let intents = GatewayIntents::GUILD_MESSAGES
    | GatewayIntents::DIRECT_MESSAGES
    | GatewayIntents::MESSAGE_CONTENT;
    
    let pg_host = env::var("PG_HOST").expect("Expected PG_HOST in the environment");
    let pg_port = env::var("PG_PORT").expect("Expected PG_PORT in the environment");
    let pg_user = env::var("PG_USER").expect("Expected PG_USER in the environment");
    let pg_pass = env::var("PG_PASS").expect("Expected PG_PASS in the environment");
    let pg_database = env::var("PG_DATABASE").expect("Expected PG_DATABASE in the environment");
    let db = Database::new(
        &pg_host,
        &pg_user,
        &pg_pass,
        &pg_database,
        pg_port.parse().expect("Port was not an unsigned integer.")
    ).await?;

    let mut client =
    Client::builder(&token, intents).event_handler(
        DZBot::new(Arc::new(db))
    ).await.expect("Err creating client");

    println!("Starting bot...");
    if let Err(why) = client.start().await {
        println!("Client error: {why:?}");
    }

    Ok(())
}