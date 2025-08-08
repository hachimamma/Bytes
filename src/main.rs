use poise::serenity_prelude as serenity;
use dotenv::dotenv;
use std::env;
use sqlx::sqlite::SqlitePoolOptions;
use serenity::{async_trait, EventHandler, Context, Message};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use chrono::{DateTime, Utc, Duration};

mod commands;
mod handlers;
use commands::*;

pub struct Data {
    pub db: sqlx::SqlitePool,
    pub act_t: Arc<Mutex<HashMap<String, UserActivity>>>,
}

#[derive(Clone)]
pub struct UserActivity {
    pub lst_rwdt: DateTime<Utc>,
    pub msgt: u32,
    pub lstrst: DateTime<Utc>,
}

impl Default for UserActivity {
    fn default() -> Self {
        Self {
            lst_rwdt: Utc::now() - Duration::hours(1),
            msgt: 0,
            lstrst: Utc::now(),
        }
    }
}

struct Handler {
    data: Arc<Data>,
}

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, _ctx: Context, msg: Message) {
        if msg.author.bot || msg.content.starts_with('/') || msg.content.starts_with('!') || msg.content.starts_with('.') {
            return;
        }

        let user_id = msg.author.id.to_string();
        let now = Utc::now();
        
        let msg_len = msg.content.len() as u32;
        let word = msg.content.split_whitespace().count() as u32;
        
        if msg_len < 3 || word < 1 {
            return;
        }

        let mut tracker = self.data.act_t.lock().await;
        let activity = tracker.entry(user_id.clone()).or_default();
        
        if now.date_naive() != activity.lstrst.date_naive() {
            activity.msgt = 0;
            activity.lstrst = now;
        }
        
        let rwdcool = Duration::minutes(1);
        if now - activity.lst_rwdt < rwdcool {
            return;
        }
        
        if activity.msgt >= 100 {
            return;
        }
        
        let rwd = rwd(msg_len, word, activity.msgt);
        
        if rwd > 0 {
            if let Err(e) = awd_actb(&self.data.db, &user_id, rwd).await {
                eprintln!("failed to award activity bits: {}", e);
                return;
            }
            
            activity.lst_rwdt = now;
            activity.msgt += 1;
            
            println!("awarded {} bits to user {} (message #{} today)", rwd, &user_id, activity.msgt);
        }
        
        drop(tracker);
    }

    // Add interaction handler for shop buttons
    async fn interaction_create(&self, ctx: Context, interaction: serenity::Interaction) {
        if let serenity::Interaction::Component(component_interaction) = interaction {
            if component_interaction.data.custom_id.starts_with("shop_buy_") {
                let item_id = component_interaction.data.custom_id.strip_prefix("shop_buy_").unwrap();
                if let Err(e) = handlers::shop_back(&ctx, &component_interaction, &self.data, item_id).await {
                    eprintln!("Error handling shop purchase: {:?}", e);
                }
            }
        }
    }
}

fn rwd(msg_len: u32, word: u32, msgt: u32) -> i64 {
    let mut rwd = 2i64;
    
    rwd += match msg_len {
        5..=15 => 1,
        16..=50 => 2,
        51..=100 => 3,
        101.. => 4,
        _ => 0,
    };
    
    rwd += match word {
        2..=5 => 1,
        6..=10 => 2,
        11..=20 => 3,
        21.. => 4,
        _ => 0,
    };
    
    let act_b = match msgt {
        0..=10 => 2,
        11..=25 => 1,
        26..=50 => 0,
        _ => -1,
    };
    rwd += act_b;
    
    if rand::random::<f32>() < 0.1 {
        rwd += 3;
    }
    
    rwd.clamp(1, 12)
}

async fn awd_actb(db: &sqlx::SqlitePool, user_id: &str, amount: i64) -> Result<(), sqlx::Error> {
    sqlx::query("INSERT OR IGNORE INTO users (id, bits) VALUES (?, 0)")
        .bind(user_id)
        .execute(db)
        .await?;
    
    sqlx::query("UPDATE users SET bits = bits + ? WHERE id = ?")
        .bind(amount)
        .bind(user_id)
        .execute(db)
        .await?;
    
    Ok(())
}

pub type Error = Box<dyn std::error::Error + Send + Sync>;

#[tokio::main]
async fn main() -> Result<(), Error> {
    dotenv().ok();
    
    let token = env::var("DISCORD_TOKEN")?;
    
    // Get the project root directory
    let current_dir = std::env::current_dir()?;
    let project_root = if current_dir.ends_with("shop") {
        current_dir.parent().unwrap().to_path_buf()
    } else {
        current_dir
    };
    
    // Set working directory to project root
    std::env::set_current_dir(&project_root)?;
    
    let db_url = env::var("DB_URL").unwrap_or("sqlite:bytes.db".into());
    
    println!("database URL: {}", &db_url);
    println!("current directory: {:?}", std::env::current_dir());
    println!("project root: {:?}", &project_root);
    
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
    
    let act_t = Arc::new(Mutex::new(HashMap::new()));
    let actt_h = Arc::clone(&act_t);
    let actt_f = Arc::clone(&act_t);
    let dbf_f = db.clone();
    
    let options = poise::FrameworkOptions {
        commands: vec![
            daily(), balance(), leaderboard(), rob(), coinflip(),
            tax(), set(), pay(), monthly(), weekly(), add(), dice(), subtract(), yearly(), shop(), additem(),
            backpack()
        ],
        ..Default::default()
    };
    
    let framework = poise::Framework::builder()
        .options(options)
        .setup(move |ctx, _ready, framework| {
            let data_clone = Data {
                db: dbf_f,
                act_t: actt_f,
            };
            Box::pin(async move {
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                Ok(data_clone)
            })
        })
        .build();
    
    let handler = Handler {
        data: Arc::new(Data {
            db,
            act_t: actt_h,
        }),
    };
    
    let mut client = serenity::ClientBuilder::new(
        token,
        serenity::GatewayIntents::non_privileged() | serenity::GatewayIntents::MESSAGE_CONTENT
    )
    .framework(framework)
    .event_handler(handler)
    .await?;
    
    println!("bot starting with improved activity rwd system!");
    println!("rwd system: pls read the code");
    client.start().await?;
    
    Ok(())
}