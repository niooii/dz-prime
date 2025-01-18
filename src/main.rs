use std::env;
mod model;
mod bot;
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
    
    let postgres_url = env::var("DATABASE_URL").expect("Expected DATABASE_URL in the environment");
    let db = Database::new(&postgres_url).await?;

    let mut client =
    Client::builder(&token, intents).event_handler(
        DZBot::new(db)
    ).await.expect("Err creating client");

    println!("Starting bot...");
    if let Err(why) = client.start().await {
        println!("Client error: {why:?}");
    }

    Ok(())
}