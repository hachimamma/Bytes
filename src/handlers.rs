//ignore warnings this works just fine

use crate::{Data, Error};
use chrono::{DateTime, Utc};
use poise::serenity_prelude::CreateEmbed;
use poise::serenity_prelude::{CreateEmbedAuthor, CreateEmbedFooter};
use num_format::{Locale, ToFormattedString};
use rand::Rng;
use sqlx::Row;

//to check for core prevelege
pub(crate) async fn is_admin(ctx: &poise::Context<'_, Data, Error>) -> bool {
    if let Some(_guild_id) = ctx.guild_id() {
        if let Some(member) = ctx.author_member().await.as_ref() {
            member.permissions.unwrap_or_default().administrator()
        } else { 
            false 
        }
    } else { 
        false 
    }
}

//sql query check for value binding (WIP)
async fn ensure_user(ctx: &poise::Context<'_, Data, Error>) -> Result<(), Error> {
    sqlx::query("INSERT OR IGNORE INTO users (id, bits) VALUES (?, 0)")
        .bind(ctx.author().id.to_string())
        .execute(&ctx.data().db)
        .await?;
    Ok(())
}

//sql query for balance binding (WIP)
pub async fn balance(ctx: poise::Context<'_, Data, Error>) -> Result<(), Error> {
    ensure_user(&ctx).await?;
    let bits: i64 = sqlx::query("SELECT bits FROM users WHERE id = ?")
        .bind(ctx.author().id.to_string())
        .fetch_one(&ctx.data().db)
        .await?
        .get("bits");

    ctx.send(poise::CreateReply::default().embed(
        CreateEmbed::new()
            .author(CreateEmbedAuthor::new(ctx.author().name.clone()).icon_url(ctx.author().avatar_url().unwrap_or_default()))
            .title("Balance")
            .description(format!("{} bits", bits.to_formatted_string(&Locale::en)))
            .footer(CreateEmbedFooter::new("Bits"))
            .thumbnail("https://cdn.discordapp.com/assets/2c21aeda16de354ba5334551a883b481.png")
            .color(0x5865F2) // Discord blurple
    )).await?;
    Ok(())
}


//sql query for lb binding (WIP)
pub async fn leaderboard(ctx: poise::Context<'_, Data, Error>) -> Result<(), Error> {
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
        CreateEmbed::new()
            .title("Top Sigmas Leaderboard")
            .description(&msg)
            .color(0x6C3483)
    )).await?;
    Ok(())
}

//counters for coinflips (A)
pub async fn coinflip(
    ctx: poise::Context<'_, Data, Error>,
    betamt: i64,
    bet_side: String,
) -> Result<(), Error> {
    ensure_user(&ctx).await?;
    
    if betamt <= 0 {
        ctx.send(poise::CreateReply::default().embed(
            CreateEmbed::new().title("Coinflip").description("Bet amount must be positive. :3")
        )).await?;
        return Ok(());
    }
    
    let bets_lower = bet_side.to_lowercase();
    if bets_lower != "heads" && bets_lower != "tails" {
        ctx.send(poise::CreateReply::default().embed(
            CreateEmbed::new().title("Coinflip").description("Bet side must be heads or tails. :3")
        )).await?;
        return Ok(());
    }
    
    let user_id = ctx.author().id.to_string();
    
    let c_bits: i64 = sqlx::query("SELECT bits FROM users WHERE id = ?")
        .bind(&user_id)
        .fetch_one(&ctx.data().db)
        .await?
        .get("bits");
    
    if c_bits < betamt {
        ctx.send(poise::CreateReply::default().embed(
            CreateEmbed::new().title("Coinflip").description("You don't have enough Bits to place this bet. :3")
        )).await?;
        return Ok(());
    }
    
    let fp_res = rand::random::<bool>();
    let fp_side = if fp_res { "heads" } else { "tails" };
    let won = bets_lower == fp_side;
    
    let delta = if won { betamt } else { -betamt };
    
    sqlx::query("UPDATE users SET bits = bits + ? WHERE id = ?")
        .bind(delta)
        .bind(&user_id)
        .execute(&ctx.data().db)
        .await?;
    
    let symbol = if fp_res { "H" } else { "T" };
    let (res_msg, embed_color) = if won {
        (format!("[{}] The coin landed on **{}**! You won {} Bits. :3", 
                symbol, fp_side, betamt), 0x00ff00)
    } else {
        (format!("[{}] The coin landed on **{}**... You lost {} Bits. :3", 
                symbol, fp_side, betamt), 0xff0000)
    };
    
    ctx.send(poise::CreateReply::default().embed(
        CreateEmbed::new()
            .title("Coinflip")
            .description(&res_msg)
            .color(embed_color)
    )).await?;
    Ok(())
}
//handle for dice (A)
pub async fn dice(
    ctx: poise::Context<'_, Data, Error>,
    betamt: i64,
    bet_side: i32,
) -> Result<(), Error> {
    ensure_user(&ctx).await?;
    
    if betamt <= 0 {
        ctx.send(poise::CreateReply::default().embed(
            CreateEmbed::new().title("Dice").description("Bet amount must be positive. :3")
        )).await?;
        return Ok(());
    }
    
    if bet_side < 1 || bet_side > 6 {
        ctx.send(poise::CreateReply::default().embed(
            CreateEmbed::new().title("Dice").description("Bet side must be between 1 and 6. :3")
        )).await?;
        return Ok(());
    }
    
    let user_id = ctx.author().id.to_string();
    
    let c_bits: i64 = sqlx::query("SELECT bits FROM users WHERE id = ?")
        .bind(&user_id)
        .fetch_one(&ctx.data().db)
        .await?
        .get("bits");
    
    if c_bits < betamt {
        ctx.send(poise::CreateReply::default().embed(
            CreateEmbed::new().title("Dice").description("You don't have enough Bits to place this bet. :3")
        )).await?;
        return Ok(());
    }
    
    let roll = rand::thread_rng().gen_range(1..=6);
    let won = roll == bet_side;
    
    let delta = if won { betamt * 5 } else { -betamt };
    
    sqlx::query("UPDATE users SET bits = bits + ? WHERE id = ?")
        .bind(delta)
        .bind(&user_id)
        .execute(&ctx.data().db)
        .await?;
    
    let (res_msg, embed_color) = if won {
        (format!("You rolled a {} and won! You won {} Bits. :3", 
                roll, betamt * 5), 0x00ff00)
    } else {
        (format!("You rolled a {} and lost... You lost {} Bits. :3", 
                roll, betamt), 0xff0000)
    };
    
    ctx.send(poise::CreateReply::default().embed(
        CreateEmbed::new()
            .title("Dice Roll")
            .description(&res_msg)
            .color(embed_color)
    )).await?;
    Ok(())
}

pub async fn pay(ctx: poise::Context<'_, Data, Error>) -> Result<(), Error> {
    ctx.send(poise::CreateReply::default().embed(
        CreateEmbed::new().title("Pay").description("Pay command needs to be updated with params. :3")
    )).await?;
    Ok(())
}

//ciunter for rob
pub async fn rob(
    ctx: poise::Context<'_, Data, Error>,
    target: poise::serenity_prelude::User,
) -> Result<(), Error> {
    ensure_user(&ctx).await?;

    if ctx.author().id == target.id {
        ctx.send(poise::CreateReply::default().embed(
            CreateEmbed::new()
                .title("Rob")
                .description("You cannot rob yourself dummy. :3")
                .color(0x6C3483)
        )).await?;
        return Ok(());
    }

    let target_id = target.id.to_string();

    sqlx::query("INSERT OR IGNORE INTO users (id, bits) VALUES (?, 0)")
        .bind(&target_id)
        .execute(&ctx.data().db)
        .await?;

    let target_bits: i64 = sqlx::query("SELECT bits FROM users WHERE id = ?")
        .bind(&target_id)
        .fetch_one(&ctx.data().db)
        .await?
        .get("bits");

    if target_bits < 200 {
        ctx.send(poise::CreateReply::default().embed(
            CreateEmbed::new()
                .title("Rob")
                .description("Target must have at least 200 Bits to be robbed. :3")
                .color(0x6C3483)
        )).await?;
        return Ok(());
    }

    let stolen = rand::thread_rng().gen_range(50..=200);
    let act_s = std::cmp::min(stolen, target_bits);

    sqlx::query("UPDATE users SET bits = bits - ? WHERE id = ?")
        .bind(act_s)
        .bind(&target_id)
        .execute(&ctx.data().db).await?;

    sqlx::query("UPDATE users SET bits = bits + ? WHERE id = ?")
        .bind(act_s)
        .bind(ctx.author().id.to_string())
        .execute(&ctx.data().db).await?;

    ctx.send(poise::CreateReply::default().embed(
        CreateEmbed::new()
            .title("Rob")
            .description(&format!("You stole {} Bits from {}. :>", act_s, target.display_name()))
            .color(0x6C3483)
    )).await?;
    Ok(())
}

//add func only for admins with admin_check above
pub async fn add(ctx: poise::Context<'_, Data, Error>) -> Result<(), Error> {
    if !is_admin(&ctx).await {
        ctx.send(poise::CreateReply::default().embed(
            CreateEmbed::new().title("Add").description("Only admins can use this command. :3")
        )).await?;
        return Ok(());
    }
    
    ctx.send(poise::CreateReply::default().embed(
        CreateEmbed::new().title("Add").description("add command is handled in commands.rs now (the proj is WIP)")
    )).await?;
    Ok(())
}

//sub func for admins same as add but -
pub async fn subtract(ctx: poise::Context<'_, Data, Error>) -> Result<(), Error> {
    if !is_admin(&ctx).await {
        ctx.send(poise::CreateReply::default().embed(
            CreateEmbed::new().title("Subtract").description("Only admins can use this command. :3")
        )).await?;
        return Ok(());
    }
    
    ctx.send(poise::CreateReply::default().embed(
        CreateEmbed::new().title("Subtract").description("subtract command is handled in commands.rs now (the proj is WIP)")
    )).await?;
    Ok(())
}

//set func for setting bal
pub async fn set(ctx: poise::Context<'_, Data, Error>) -> Result<(), Error> {
    if !is_admin(&ctx).await {
        ctx.send(poise::CreateReply::default().embed(
            CreateEmbed::new().title("Set").description("Only admins can use this command. :3")
        )).await?;
        return Ok(());
    }
    
    ctx.send(poise::CreateReply::default().embed(
        CreateEmbed::new().title("Set").description("set command is handled in commands.rs now (the proj is wip)")
    )).await?;
    Ok(())
}

//tax counter for admins
pub async fn tax(
    ctx: poise::Context<'_, Data, Error>,
    target: poise::serenity_prelude::User,
    percent: f64,
) -> Result<(), Error> {
    if !is_admin(&ctx).await {
        ctx.send(poise::CreateReply::default().embed(
            CreateEmbed::new().title("Tax").description("Only admins can use this command. :>")
        )).await?;
        return Ok(());
    }
    
    if percent < 0.0 || percent > 100.0 {
        ctx.send(poise::CreateReply::default().embed(
            CreateEmbed::new().title("Tax").description("Tax percent must be between 0 and 100. :3")
        )).await?;
        return Ok(());
    }
    
    let target_id = target.id.to_string();
    
    sqlx::query("INSERT OR IGNORE INTO users (id, bits) VALUES (?, 0)")
        .bind(&target_id)
        .execute(&ctx.data().db)
        .await?;
    
    let balance: i64 = sqlx::query("SELECT bits FROM users WHERE id = ?")
        .bind(&target_id)
        .fetch_one(&ctx.data().db)
        .await?
        .get("bits");
    
    let tax_amount = ((balance as f64 / 100.0) * percent) as i64;
    let new_balance = balance - tax_amount;
    
    sqlx::query("UPDATE users SET bits = ? WHERE id = ?")
        .bind(new_balance)
        .bind(&target_id)
        .execute(&ctx.data().db).await?;
    
    ctx.send(poise::CreateReply::default().embed(
        CreateEmbed::new().title("Tax").description(&format!(
            "Taxed {}: {}%\nTax: {} Bits\nNew Balance: {} Bits. :3", 
            target.display_name(), percent, tax_amount, new_balance
        ))
    )).await?;
    Ok(())
}


//daily counter
pub async fn daily(ctx: poise::Context<'_, Data, Error>) -> Result<(), Error> {
    ensure_user(&ctx).await?;
    let user_id = ctx.author().id.to_string();
    let now = Utc::now().to_rfc3339();

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
                    CreateEmbed::new()
                        .title("Daily")
                        .description(&format!("You already claimed your daily! Try again in {} hours. :3", hours_left))
                        .color(0x6C3483) // Dark purple
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
        CreateEmbed::new()
            .title("Daily")
            .description(&format!("You claimed your daily reward of {} Bits! :D", reward))
            .color(0x6C3483) // Dark purple
    )).await?;
    Ok(())
}

//weekly counter for all, pls fix time if u can
pub async fn weekly(ctx: poise::Context<'_, Data, Error>) -> Result<(), Error> {
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
                    CreateEmbed::new().title("Weekly").description(&format!("You already claimed your weekly! Try again in {} days. :3", days_left))
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

//monthly counter
pub async fn monthly(ctx: poise::Context<'_, Data, Error>) -> Result<(), Error> {
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
                    CreateEmbed::new()
                        .title("Monthly")
                        .description(&format!("You already claimed your monthly! Try again in {} days. :3", days_left))
                        .color(0x6C3483) // Dark purple
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
        CreateEmbed::new()
            .title("Monthly")
            .description(&format!("You claimed your monthly reward of {} Bits! 8)", reward))
            .color(0x6C3483)
    )).await?;
    Ok(())
}

//pls fix this thing, its really bad logic
pub async fn yearly(ctx: poise::Context<'_, Data, Error>) -> Result<(), Error> {
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
                    CreateEmbed::new().title("Yearly").description(&format!("You already claimed your yearly! Try again in {} days. :3", days_left))
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