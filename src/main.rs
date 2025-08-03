use poise::serenity_prelude as serenity;
use dotenv::dotenv;
use std::env;
use sqlx::sqlite::SqlitePoolOptions;

mod commands;
mod handlers;
use commands::*;

pub struct Data {
    pub db: sqlx::SqlitePool,
}

pub type Error = Box<dyn std::error::Error + Send + Sync>;

#[tokio::main]
async fn main() -> Result<(), Error> {
    dotenv().ok();
    
    let token = env::var("DISCORD_TOKEN")?;
    let db_url = env::var("DB_URL").unwrap_or("sqlite:database.db".into());
    
    // Debug: print the database URL
    println!("Database URL: {}", &db_url);
    println!("Current directory: {:?}", std::env::current_dir());
    
    let db = SqlitePoolOptions::new()
        .connect(&db_url)
        .await?;
    
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS users (
            id TEXT PRIMARY KEY,
            bits INTEGER NOT NULL DEFAULT 0,
            last_daily TEXT,
            last_work TEXT,
            last_weekly TEXT,
            last_monthly TEXT,
            last_yearly TEXT
        )"
    )
    .execute(&db)
    .await?;
    
    let options = poise::FrameworkOptions {
        commands: vec![
            daily(), work(), balance(), leaderboard(), rob(), bitflip(),
            tax(), set(), pay(), monthly(), weekly(), add(), dice(), subtract(), yearly()
        ],
        ..Default::default()
    };
    
    let framework = poise::Framework::builder()
        .options(options)
        .setup(|ctx, _ready, framework| {
            Box::pin(async move {
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                Ok(Data { db })
            })
        })
        .build();
    
    let mut client = serenity::ClientBuilder::new(
        token,
        serenity::GatewayIntents::non_privileged() | serenity::GatewayIntents::MESSAGE_CONTENT
    )
    .framework(framework)
    .await?;
    
    client.start().await?;
    Ok(())
}