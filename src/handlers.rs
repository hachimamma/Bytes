use crate::{Data, Error};
use chrono::{DateTime, Utc};
use poise::serenity_prelude::CreateEmbed;
use rand::Rng;
use sqlx::Row;

// Fixed macro - returns Ok(()) instead of trying to use ?
macro_rules! embed_reply {
    ($ctx:expr, $title:expr, $desc:expr) => {
        {
            $ctx.send(poise::CreateReply::default().embed(
                CreateEmbed::new().title($title).description($desc)
            )).await?;
            Ok(())
        }
    };
}

// Fixed permission check - check in guild context
async fn is_mod(ctx: &poise::Context<'_, Data, Error>) -> bool {
    if let Some(_guild_id) = ctx.guild_id() {
        if let Some(member) = ctx.author_member().await.as_ref() {
            return member.permissions.unwrap_or_default().administrator();
        }
    }
    false
}

async fn ensure_user(ctx: &poise::Context<'_, Data, Error>) -> Result<(), Error> {
    sqlx::query("INSERT OR IGNORE INTO users (id, bits) VALUES (?, 0)")
        .bind(ctx.author().id.to_string())
        .execute(&ctx.data().db)
        .await?;
    Ok(())
}

pub async fn handle_balance(ctx: poise::Context<'_, Data, Error>) -> Result<(), Error> {
    ensure_user(&ctx).await?;
    let bits: i64 = sqlx::query("SELECT bits FROM users WHERE id = ?")
        .bind(ctx.author().id.to_string())
        .fetch_one(&ctx.data().db)
        .await?
        .get("bits");

    ctx.send(poise::CreateReply::default().embed(
        CreateEmbed::new().title("Balance").description(&format!("You have {} Bits.", bits))
    )).await?;
    Ok(())
}

pub async fn handle_leaderboard(ctx: poise::Context<'_, Data, Error>) -> Result<(), Error> {
    let rows = sqlx::query("SELECT id, bits FROM users ORDER BY bits DESC LIMIT 10")
        .fetch_all(&ctx.data().db)
        .await?;

    let mut msg = String::new();
    for (i, row) in rows.iter().enumerate() {
        let id: String = row.get("id");
        let bits: i64 = row.get("bits");
        msg.push_str(&format!("{}. <@{}> - {} Bits\n", i + 1, id, bits));
    }

    ctx.send(poise::CreateReply::default().embed(
        CreateEmbed::new().title("Leaderboard").description(&msg)
    )).await?;
    Ok(())
}

pub async fn handle_bitflip(
    ctx: poise::Context<'_, Data, Error>,
    bet_amount: i64,
    bet_side: String,
) -> Result<(), Error> {
    ensure_user(&ctx).await?;
    
    if bet_amount <= 0 {
        ctx.send(poise::CreateReply::default().embed(
            CreateEmbed::new().title("Bitflip").description("Bet amount must be positive!")
        )).await?;
        return Ok(());
    }
    
    let bet_side_lower = bet_side.to_lowercase();
    if bet_side_lower != "heads" && bet_side_lower != "tails" {
        ctx.send(poise::CreateReply::default().embed(
            CreateEmbed::new().title("Bitflip").description("Bet side must be 'heads' or 'tails'!")
        )).await?;
        return Ok(());
    }
    
    let user_id = ctx.author().id.to_string();
    
    // Check if user has enough bits
    let current_bits: i64 = sqlx::query("SELECT bits FROM users WHERE id = ?")
        .bind(&user_id)
        .fetch_one(&ctx.data().db)
        .await?
        .get("bits");
    
    if current_bits < bet_amount {
        ctx.send(poise::CreateReply::default().embed(
            CreateEmbed::new().title("Bitflip").description("You don't have enough Bits to place this bet!")
        )).await?;
        return Ok(());
    }
    
    let flip_result = rand::random::<bool>();
    let flip_side = if flip_result { "heads" } else { "tails" };
    let won = bet_side_lower == flip_side;
    
    let delta = if won { bet_amount } else { -bet_amount }; // 2x payout (double your bet)
    
    sqlx::query("UPDATE users SET bits = bits + ? WHERE id = ?")
        .bind(delta)
        .bind(&user_id)
        .execute(&ctx.data().db)
        .await?;
    
    let symbol = if flip_result { "H" } else { "T" };
    let result_msg = if won {
        format!("[{}] The coin landed on **{}**! You bet {} Bits on {} and WON {} Bits!", 
                symbol, flip_side, bet_amount, bet_side_lower, bet_amount * 2)
    } else {
        format!("[{}] The coin landed on **{}**... You bet {} Bits on {} and lost {} Bits.", 
                symbol, flip_side, bet_amount, bet_side_lower, bet_amount)
    };
    
    ctx.send(poise::CreateReply::default().embed(
        CreateEmbed::new().title("Bitflip").description(&result_msg)
    )).await?;
    Ok(())
}

pub async fn handle_dice(
    ctx: poise::Context<'_, Data, Error>,
    bet_amount: i64,
    bet_side: i32,
) -> Result<(), Error> {
    ensure_user(&ctx).await?;
    
    if bet_amount <= 0 {
        ctx.send(poise::CreateReply::default().embed(
            CreateEmbed::new().title("Dice").description("Bet amount must be positive!")
        )).await?;
        return Ok(());
    }
    
    if bet_side < 1 || bet_side > 6 {
        ctx.send(poise::CreateReply::default().embed(
            CreateEmbed::new().title("Dice").description("Bet side must be between 1 and 6!")
        )).await?;
        return Ok(());
    }
    
    let user_id = ctx.author().id.to_string();
    
    // Check if user has enough bits
    let current_bits: i64 = sqlx::query("SELECT bits FROM users WHERE id = ?")
        .bind(&user_id)
        .fetch_one(&ctx.data().db)
        .await?
        .get("bits");
    
    if current_bits < bet_amount {
        ctx.send(poise::CreateReply::default().embed(
            CreateEmbed::new().title("Dice").description("You don't have enough Bits to place this bet!")
        )).await?;
        return Ok(());
    }
    
    let roll = rand::thread_rng().gen_range(1..=6);
    let won = roll == bet_side;
    
    let delta = if won { bet_amount * 5 } else { -bet_amount }; // 5x payout if win
    
    sqlx::query("UPDATE users SET bits = bits + ? WHERE id = ?")
        .bind(delta)
        .bind(&user_id)
        .execute(&ctx.data().db)
        .await?;
    
    let result_msg = if won {
        format!(":) You rolled a {} and WON! You bet {} Bits on {} and won {} Bits!", 
                roll, bet_amount, bet_side, bet_amount * 5)
    } else {
        format!(":( You rolled a {} and lost... You bet {} Bits on {} and lost {} Bits.", 
                roll, bet_amount, bet_side, bet_amount)
    };
    
    ctx.send(poise::CreateReply::default().embed(
        CreateEmbed::new().title("Dice Roll").description(&result_msg)
    )).await?;
    Ok(())
}

pub async fn handle_pay(ctx: poise::Context<'_, Data, Error>) -> Result<(), Error> {
    // For newer Poise, we don't need to extract from interaction manually
    // The parameters are passed through the command function
    ctx.send(poise::CreateReply::default().embed(
        CreateEmbed::new().title("Pay").description("Pay command needs to be updated with proper parameter passing!")
    )).await?;
    Ok(())
}

pub async fn handle_rob(
    ctx: poise::Context<'_, Data, Error>,
    target: poise::serenity_prelude::User,
) -> Result<(), Error> {
    ensure_user(&ctx).await?;
    
    // Check if user is trying to rob themselves
    if ctx.author().id == target.id {
        ctx.send(poise::CreateReply::default().embed(
            CreateEmbed::new().title("Rob").description("You cannot rob yourself!")
        )).await?;
        return Ok(());
    }
    
    let target_id = target.id.to_string();
    
    // Ensure target user exists
    sqlx::query("INSERT OR IGNORE INTO users (id, bits) VALUES (?, 0)")
        .bind(&target_id)
        .execute(&ctx.data().db)
        .await?;
    
    // Get target's balance
    let target_bits: i64 = sqlx::query("SELECT bits FROM users WHERE id = ?")
        .bind(&target_id)
        .fetch_one(&ctx.data().db)
        .await?
        .get("bits");
    
    if target_bits < 200 {
        ctx.send(poise::CreateReply::default().embed(
            CreateEmbed::new().title("Rob").description("Target must have at least 200 Bits to be robbed!")
        )).await?;
        return Ok(());
    }
    
    // Random steal amount (50-200)
    let stolen = rand::thread_rng().gen_range(50..=200);
    let actual_stolen = std::cmp::min(stolen, target_bits);
    
    // Transfer bits
    sqlx::query("UPDATE users SET bits = bits - ? WHERE id = ?")
        .bind(actual_stolen)
        .bind(&target_id)
        .execute(&ctx.data().db).await?;
    
    sqlx::query("UPDATE users SET bits = bits + ? WHERE id = ?")
        .bind(actual_stolen)
        .bind(ctx.author().id.to_string())
        .execute(&ctx.data().db).await?;
    
    ctx.send(poise::CreateReply::default().embed(
        CreateEmbed::new().title("Rob").description(&format!("You stole {} Bits from {}!", actual_stolen, target.display_name()))
    )).await?;
    Ok(())
}

pub async fn handle_add(ctx: poise::Context<'_, Data, Error>) -> Result<(), Error> {
    if !is_mod(&ctx).await {
        ctx.send(poise::CreateReply::default().embed(
            CreateEmbed::new().title("Add").description("Only mods can use this.")
        )).await?;
        return Ok(());
    }
    
    ctx.send(poise::CreateReply::default().embed(
        CreateEmbed::new().title("Add").description("Add command needs parameter passing!")
    )).await?;
    Ok(())
}

pub async fn handle_subtract(ctx: poise::Context<'_, Data, Error>) -> Result<(), Error> {
    if !is_mod(&ctx).await {
        ctx.send(poise::CreateReply::default().embed(
            CreateEmbed::new().title("Subtract").description("Only mods can use this.")
        )).await?;
        return Ok(());
    }
    
    ctx.send(poise::CreateReply::default().embed(
        CreateEmbed::new().title("Subtract").description("Subtract command needs parameter passing!")
    )).await?;
    Ok(())
}

pub async fn handle_set(ctx: poise::Context<'_, Data, Error>) -> Result<(), Error> {
    if !is_mod(&ctx).await {
        ctx.send(poise::CreateReply::default().embed(
            CreateEmbed::new().title("Set").description("Only mods can use this.")
        )).await?;
        return Ok(());
    }
    
    ctx.send(poise::CreateReply::default().embed(
        CreateEmbed::new().title("Set").description("Set command needs parameter passing!")
    )).await?;
    Ok(())
}

pub async fn handle_tax(
    ctx: poise::Context<'_, Data, Error>,
    target: poise::serenity_prelude::User,
    percentage: f64,
) -> Result<(), Error> {
    if !is_mod(&ctx).await {
        ctx.send(poise::CreateReply::default().embed(
            CreateEmbed::new().title("Tax").description("Only admins can use this command!")
        )).await?;
        return Ok(());
    }
    
    if percentage < 0.0 || percentage > 100.0 {
        ctx.send(poise::CreateReply::default().embed(
            CreateEmbed::new().title("Tax").description("Tax percentage must be between 0 and 100!")
        )).await?;
        return Ok(());
    }
    
    let target_id = target.id.to_string();
    
    // Ensure target user exists
    sqlx::query("INSERT OR IGNORE INTO users (id, bits) VALUES (?, 0)")
        .bind(&target_id)
        .execute(&ctx.data().db)
        .await?;
    
    // Get target's balance
    let balance: i64 = sqlx::query("SELECT bits FROM users WHERE id = ?")
        .bind(&target_id)
        .fetch_one(&ctx.data().db)
        .await?
        .get("bits");
    
    // Calculate tax
    let tax_amount = ((balance as f64 / 100.0) * percentage) as i64;
    let new_balance = balance - tax_amount;
    
    // Update balance
    sqlx::query("UPDATE users SET bits = ? WHERE id = ?")
        .bind(new_balance)
        .bind(&target_id)
        .execute(&ctx.data().db).await?;
    
    ctx.send(poise::CreateReply::default().embed(
        CreateEmbed::new().title("Tax").description(&format!(
            "Taxed {}: {}%\nTax: {} Bits\nNew Balance: {} Bits", 
            target.display_name(), percentage, tax_amount, new_balance
        ))
    )).await?;
    Ok(())
}

pub async fn handle_daily(ctx: poise::Context<'_, Data, Error>) -> Result<(), Error> {
    ensure_user(&ctx).await?;
    let user_id = ctx.author().id.to_string();
    let now = Utc::now().to_rfc3339();
    
    // Check last daily claim
    let last_daily: Option<String> = sqlx::query("SELECT last_daily FROM users WHERE id = ?")
        .bind(&user_id)
        .fetch_one(&ctx.data().db)
        .await?
        .get("last_daily");
    
    if let Some(last_time_str) = last_daily {
        if let Ok(last_time) = DateTime::parse_from_rfc3339(&last_time_str) {
            let time_diff = Utc::now().signed_duration_since(last_time);
            if time_diff.num_hours() < 24 {
                let hours_left = 24 - time_diff.num_hours();
                ctx.send(poise::CreateReply::default().embed(
                    CreateEmbed::new().title("Daily").description(&format!("You already claimed your daily! Try again in {} hours.", hours_left))
                )).await?;
                return Ok(());
            }
        }
    }
    
    let reward = 100;
    sqlx::query("UPDATE users SET bits = bits + ?, last_daily = ? WHERE id = ?")
        .bind(reward)
        .bind(&now)
        .bind(&user_id)
        .execute(&ctx.data().db)
        .await?;
    
    ctx.send(poise::CreateReply::default().embed(
        CreateEmbed::new().title("Daily").description(&format!("You claimed your daily reward of {} Bits! :D", reward))
    )).await?;
    Ok(())
}

pub async fn handle_work(ctx: poise::Context<'_, Data, Error>) -> Result<(), Error> {
    ensure_user(&ctx).await?;
    let user_id = ctx.author().id.to_string();
    let now = Utc::now().to_rfc3339();
    
    // Check last work
    let last_work: Option<String> = sqlx::query("SELECT last_work FROM users WHERE id = ?")
        .bind(&user_id)
        .fetch_one(&ctx.data().db)
        .await?
        .get("last_work");
    
    if let Some(last_time_str) = last_work {
        if let Ok(last_time) = DateTime::parse_from_rfc3339(&last_time_str) {
            let time_diff = Utc::now().signed_duration_since(last_time);
            if time_diff.num_hours() < 1 {
                let minutes_left = 60 - time_diff.num_minutes();
                ctx.send(poise::CreateReply::default().embed(
                    CreateEmbed::new().title("Work").description(&format!("You need to wait {} minutes before working again!", minutes_left))
                )).await?;
                return Ok(());
            }
        }
    }
    
    let reward = rand::thread_rng().gen_range(20..=80);
    sqlx::query("UPDATE users SET bits = bits + ?, last_work = ? WHERE id = ?")
        .bind(reward)
        .bind(&now)
        .bind(&user_id)
        .execute(&ctx.data().db)
        .await?;
    
    ctx.send(poise::CreateReply::default().embed(
        CreateEmbed::new().title("Work").description(&format!("You worked hard and earned {} Bits! B)", reward))
    )).await?;
    Ok(())
}

pub async fn handle_weekly(ctx: poise::Context<'_, Data, Error>) -> Result<(), Error> {
    ensure_user(&ctx).await?;
    let user_id = ctx.author().id.to_string();
    let now = Utc::now().to_rfc3339();
    
    let last_weekly: Option<String> = sqlx::query("SELECT last_weekly FROM users WHERE id = ?")
        .bind(&user_id)
        .fetch_one(&ctx.data().db)
        .await?
        .get("last_weekly");
    
    if let Some(last_time_str) = last_weekly {
        if let Ok(last_time) = DateTime::parse_from_rfc3339(&last_time_str) {
            let time_diff = Utc::now().signed_duration_since(last_time);
            if time_diff.num_days() < 7 {
                let days_left = 7 - time_diff.num_days();
                ctx.send(poise::CreateReply::default().embed(
                    CreateEmbed::new().title("Weekly").description(&format!("You already claimed your weekly! Try again in {} days.", days_left))
                )).await?;
                return Ok(());
            }
        }
    }
    
    let reward = 500;
    sqlx::query("UPDATE users SET bits = bits + ?, last_weekly = ? WHERE id = ?")
        .bind(reward)
        .bind(&now)
        .bind(&user_id)
        .execute(&ctx.data().db)
        .await?;
    
    ctx.send(poise::CreateReply::default().embed(
        CreateEmbed::new().title("Weekly").description(&format!("You claimed your weekly reward of {} Bits! :P", reward))
    )).await?;
    Ok(())
}

pub async fn handle_monthly(ctx: poise::Context<'_, Data, Error>) -> Result<(), Error> {
    ensure_user(&ctx).await?;
    let user_id = ctx.author().id.to_string();
    let now = Utc::now().to_rfc3339();
    
    let last_monthly: Option<String> = sqlx::query("SELECT last_monthly FROM users WHERE id = ?")
        .bind(&user_id)
        .fetch_one(&ctx.data().db)
        .await?
        .get("last_monthly");
    
    if let Some(last_time_str) = last_monthly {
        if let Ok(last_time) = DateTime::parse_from_rfc3339(&last_time_str) {
            let time_diff = Utc::now().signed_duration_since(last_time);
            if time_diff.num_days() < 30 {
                let days_left = 30 - time_diff.num_days();
                ctx.send(poise::CreateReply::default().embed(
                    CreateEmbed::new().title("Monthly").description(&format!("You already claimed your monthly! Try again in {} days.", days_left))
                )).await?;
                return Ok(());
            }
        }
    }
    
    let reward = 2000;
    sqlx::query("UPDATE users SET bits = bits + ?, last_monthly = ? WHERE id = ?")
        .bind(reward)
        .bind(&now)
        .bind(&user_id)
        .execute(&ctx.data().db)
        .await?;
    
    ctx.send(poise::CreateReply::default().embed(
        CreateEmbed::new().title("Monthly").description(&format!("You claimed your monthly reward of {} Bits! 8)", reward))
    )).await?;
    Ok(())
}

pub async fn handle_yearly(ctx: poise::Context<'_, Data, Error>) -> Result<(), Error> {
    ensure_user(&ctx).await?;
    let user_id = ctx.author().id.to_string();
    let now = Utc::now().to_rfc3339();
    
    let last_yearly: Option<String> = sqlx::query("SELECT last_yearly FROM users WHERE id = ?")
        .bind(&user_id)
        .fetch_one(&ctx.data().db)
        .await?
        .get("last_yearly");
    
    if let Some(last_time_str) = last_yearly {
        if let Ok(last_time) = DateTime::parse_from_rfc3339(&last_time_str) {
            let time_diff = Utc::now().signed_duration_since(last_time);
            if time_diff.num_days() < 365 {
                let days_left = 365 - time_diff.num_days();
                ctx.send(poise::CreateReply::default().embed(
                    CreateEmbed::new().title("Yearly").description(&format!("You already claimed your yearly! Try again in {} days.", days_left))
                )).await?;
                return Ok(());
            }
        }
    }
    
    let reward = 25000;
    sqlx::query("UPDATE users SET bits = bits + ?, last_yearly = ? WHERE id = ?")
        .bind(reward)
        .bind(&now)
        .bind(&user_id)
        .execute(&ctx.data().db)
        .await?;
    
    ctx.send(poise::CreateReply::default().embed(
        CreateEmbed::new().title("Yearly").description(&format!("You claimed your yearly reward of {} Bits! ^_^", reward))
    )).await?;
    Ok(())
}