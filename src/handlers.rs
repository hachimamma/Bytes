//ignore warnings this works just fine

use crate::{Data, Error};
use chrono::{DateTime, Utc};
use poise::serenity_prelude::CreateEmbed;
use poise::serenity_prelude::{CreateEmbedAuthor, CreateEmbedFooter, CreateActionRow, CreateButton, ButtonStyle};
use poise::serenity_prelude::CacheHttp;
use num_format::{Locale, ToFormattedString};
use rand::Rng;
use serenity::all::Mentionable;
use sqlx::Row;
use serde::{Deserialize, Serialize};

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

//handle for balance
pub async fn balance(ctx: poise::Context<'_, Data, Error>) -> Result<(), Error> {
    ensure_user(&ctx).await?;
    
    let botav = ctx.cache().current_user().avatar_url().unwrap_or_default();
    
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
            .footer(CreateEmbedFooter::new("Bytes").icon_url(botav))
            .thumbnail("https://cdn.discordapp.com/assets/2c21aeda16de354ba5334551a883b481.png")
            .color(0x5865F2)
    )).await?;
    Ok(())
}

// handle for lb
pub async fn leaderboard(ctx: poise::Context<'_, Data, Error>) -> Result<(), Error> {
    let rows = sqlx::query("SELECT id, bits FROM users ORDER BY bits DESC LIMIT 10")
        .fetch_all(&ctx.data().db)
        .await?;
    
    let mut desc = String::new();
    
    for (i, row) in rows.iter().enumerate() {
        let id: String = row.get("id");
        let bits: i64 = row.get("bits");
        
        desc.push_str(&format!(
            "{}.) <@{}> - **{} bits**\n", 
            i + 1, 
            id, 
            bits
        ));
    }
    
    if desc.is_empty() {
        desc = "No users found on the leaderboard yet! :3".to_string();
    }
    
    desc.push_str("\nPage 1/1");
    
    let comps = vec![CreateActionRow::Buttons(vec![
        CreateButton::new("lb_back")
            .label("Back")
            .style(ButtonStyle::Secondary)
            .disabled(true),
        CreateButton::new("lb_next")
            .label("Next")
            .style(ButtonStyle::Secondary)
            .disabled(true),
    ])];

    ctx.send(poise::CreateReply::default()
        .embed(
            CreateEmbed::new()
                .title("Top Sigmas")
                .description(&desc)
                .color(0x5865F2)
                .footer(CreateEmbedFooter::new("Bits Leaderboard"))
                .timestamp(chrono::Utc::now())
        )
        .components(comps)
    ).await?;
    
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

pub async fn pay(
    ctx: poise::Context<'_, Data, Error>,
    recipient: serenity::all::User,
    amt: i64,
) -> Result<(), Error> {
    if amt <= 0 {
        ctx.send(poise::CreateReply::default().embed(
            CreateEmbed::new()
                .title("Pay")
                .description("Amount must be positive :3.")
                .color(0x7289DA),
        )).await?;
        return Ok(());
    }

    let sender = ctx.author();
    let sender_id = sender.id.to_string();
    let rec_id = recipient.id.to_string();

    sqlx::query("INSERT OR IGNORE INTO users (id, bits) VALUES (?, 0)")
        .bind(&sender_id)
        .execute(&ctx.data().db)
        .await?;
    sqlx::query("INSERT OR IGNORE INTO users (id, bits) VALUES (?, 0)")
        .bind(&rec_id)
        .execute(&ctx.data().db)
        .await?;

    let sender_bits: i64 = sqlx::query("SELECT bits FROM users WHERE id = ?")
        .bind(&sender_id)
        .fetch_one(&ctx.data().db)
        .await?
        .get("bits");

    if sender_bits < amt {
        ctx.send(poise::CreateReply::default().embed(
            CreateEmbed::new()
                .title("Pay")
                .description("You don't have enough Bits!")
                .color(0x7289DA),
        )).await?;
        return Ok(());
    }

    sqlx::query("UPDATE users SET bits = bits - ? WHERE id = ?")
        .bind(amt)
        .bind(&sender_id)
        .execute(&ctx.data().db)
        .await?;
    sqlx::query("UPDATE users SET bits = bits + ? WHERE id = ?")
        .bind(amt)
        .bind(&rec_id)
        .execute(&ctx.data().db)
        .await?;

    let au_ub: i64 = sqlx::query("SELECT bits FROM users WHERE id = ?")
        .bind(&sender_id)
        .fetch_one(&ctx.data().db)
        .await?
        .get("bits");

    let rec_ub: i64 = sqlx::query("SELECT bits FROM users WHERE id = ?")
        .bind(&rec_id)
        .fetch_one(&ctx.data().db)
        .await?
        .get("bits");

    let botuser = ctx.serenity_context().http.get_current_user().await?;
    let guild_id = ctx.guild_id().unwrap();
    let recmem = guild_id.member(&ctx.serenity_context().http, recipient.id).await?;
    let rec_nick = recmem.nick.clone().unwrap_or_else(|| recipient.name.clone());

    let send_gl = sender.global_name.clone().unwrap_or_else(|| sender.name.clone());
    let rec_gl = recipient.global_name.clone().unwrap_or_else(|| recipient.name.clone());

    let embed = CreateEmbed::new()
        .author(CreateEmbedAuthor::new(&rec_nick)
            .icon_url(recipient.avatar_url().unwrap_or_default()))
        .title("Transaction Complete")
        .description(format!("<@{}> paid {} Bits to <@{}>", sender.id, amt, recipient.id))
        .field(
            format!("**{}'s balance:**", send_gl),
            format!("{} Bits", au_ub),
            false,
        )
        .field(
            format!("**{}'s balance:**", rec_gl),
            format!("{} Bits", rec_ub),
            false,
        )
        .footer(CreateEmbedFooter::new("Bytes")
            .icon_url(botuser.avatar_url().unwrap_or_default()))
        .color(0x7289DA);

    ctx.send(poise::CreateReply::default().embed(embed)).await?;

    Ok(())
}

//counter for rob
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
                .color(0x7289DA),
        )).await?;
        return Ok(());
    }

    let target_id = target.id.to_string();
    let robber_id = ctx.author().id.to_string();

    sqlx::query("INSERT OR IGNORE INTO users (id, bits) VALUES (?, 0)")
        .bind(&target_id)
        .execute(&ctx.data().db)
        .await?;

    sqlx::query("INSERT OR IGNORE INTO users (id, bits) VALUES (?, 0)")
        .bind(&robber_id)
        .execute(&ctx.data().db)
        .await?;

    let target_bits: i64 = sqlx::query("SELECT bits FROM users WHERE id = ?")
        .bind(&target_id)
        .fetch_one(&ctx.data().db)
        .await?
        .get("bits");

    let au_bits: i64 = sqlx::query("SELECT bits FROM users WHERE id = ?")
        .bind(&robber_id)
        .fetch_one(&ctx.data().db)
        .await?
        .get("bits");

    if target_bits < 200 {
        ctx.send(poise::CreateReply::default().embed(
            CreateEmbed::new()
                .title("Rob")
                .description("Target must have at least 200 Bits to be robbed. :3")
                .color(0x7289DA),
        )).await?;
        return Ok(());
    }

    let success = rand::thread_rng().gen_bool(0.35);

    let botuser = ctx.serenity_context().http.get_current_user().await?;
    let guild_id = ctx.guild_id().unwrap();

    let _vmem = guild_id.member(&ctx.serenity_context().http, target.id).await?;
    let au_mem = guild_id.member(&ctx.serenity_context().http, ctx.author().id).await?;

    let au_dn = au_mem.nick.clone().unwrap_or_else(|| ctx.author().global_name.clone().unwrap_or(ctx.author().name.clone()));

    let (embed_tt, embed_desc, amt_ff, target_ff);

    if success {
        let stolen = rand::thread_rng().gen_range(50..=200);
        let act_s = std::cmp::min(stolen, target_bits);

        sqlx::query("UPDATE users SET bits = bits - ? WHERE id = ?")
            .bind(act_s)
            .bind(&target_id)
            .execute(&ctx.data().db)
            .await?;

        sqlx::query("UPDATE users SET bits = bits + ? WHERE id = ?")
            .bind(act_s)
            .bind(&robber_id)
            .execute(&ctx.data().db)
            .await?;

        let scs_txt = [
            "You successfully scammed the wrong person who turns out to be an old lady for money. Are you happy with what you have done?",
            "You broke into their piggy bank while they were crying. You monster.",
            "You mugged a clown and now the circus is after you.",
            "You stole from a ninja, but somehow got away... this time.",
            "You tricked them with a fake charity and ran with the money.",
            "Seems like someone was buying items using your balance. Better go find out who..."
        ];
        let selected = scs_txt[rand::thread_rng().gen_range(0..scs_txt.len())];

        embed_tt = "Robbery Successful";
        embed_desc = selected.to_string();
        amt_ff = format!("{} bits", act_s);
        target_ff = format!("<@{}>", target.id);
    } else {
        let loss = rand::thread_rng().gen_range(25..=150);
        let act_l = std::cmp::min(loss, au_bits);

        sqlx::query("UPDATE users SET bits = bits - ? WHERE id = ?")
            .bind(act_l)
            .bind(&robber_id)
            .execute(&ctx.data().db)
            .await?;

        sqlx::query("UPDATE users SET bits = bits + ? WHERE id = ?")
            .bind(act_l)
            .bind(&target_id)
            .execute(&ctx.data().db)
            .await?;

        let fail_txt = [
            "You got caught red-handed and had to pay the victim for damages.",
            "The police intervened and fined you on the spot.",
            "Turns out it was a trap. You lost money and your dignity.",
            "The victim fought back and made you drop your wallet.",
            "You tripped during the heist and had to compensate the target."
        ];
        let selected = fail_txt[rand::thread_rng().gen_range(0..fail_txt.len())];

        embed_tt = "Robbery Failed";
        embed_desc = selected.to_string();
        amt_ff = format!("{} bits", act_l);
        target_ff = format!("<@{}>", target.id);
    }

    let embed = CreateEmbed::new()
        .title(embed_tt)
        .description(embed_desc)
        .field("Amount", amt_ff, true)
        .field("Victim", target_ff, true)
        .author(poise::serenity_prelude::CreateEmbedAuthor::new(&au_dn)
            .icon_url(ctx.author().avatar_url().unwrap_or_default()))
        .footer(poise::serenity_prelude::CreateEmbedFooter::new("Bytes")
            .icon_url(botuser.avatar_url().unwrap_or_default()))
        .color(0x7289DA);

    ctx.send(poise::CreateReply::default().embed(embed)).await?;

    Ok(())
}

// add func for admins
pub async fn add(
    ctx: poise::Context<'_, Data, Error>,
    target: poise::serenity_prelude::User,
    amount: u64,
) -> Result<(), Error> {
    if !is_admin(&ctx).await {
        ctx.send(poise::CreateReply::default().embed(
            CreateEmbed::new()
                .title("Add")
                .description("Only admins can use this command. :3")
                .color(0x7289DA),
        )).await?;
        return Ok(());
    }

    let target_id = target.id.to_string();

    sqlx::query("INSERT OR IGNORE INTO users (id, bits) VALUES (?, 0)")
        .bind(&target_id)
        .execute(&ctx.data().db)
        .await?;

    sqlx::query("UPDATE users SET bits = bits + ? WHERE id = ?")
        .bind(amount as i64)
        .bind(&target_id)
        .execute(&ctx.data().db)
        .await?;

    let updated_bits: i64 = sqlx::query("SELECT bits FROM users WHERE id = ?")
        .bind(&target_id)
        .fetch_one(&ctx.data().db)
        .await?
        .get("bits");

    let botuser = ctx.serenity_context().http.get_current_user().await?;

    let embed = CreateEmbed::new()
        .author(
            poise::serenity_prelude::CreateEmbedAuthor::new(
                target.global_name.clone().unwrap_or_else(|| target.name.clone())
            ).icon_url(target.avatar_url().unwrap_or_default())
        )
        .title("Transaction Complete")
        .description(format!(
            "<@{}> added {} bits to <@{}>",
            ctx.author().id,
            amount,
            target.id
        ))
        .field("Balance", format!("{} bits", updated_bits), false)
        .footer(
            poise::serenity_prelude::CreateEmbedFooter::new("Bytes")
                .icon_url(botuser.avatar_url().unwrap_or_default())
        )
        .color(0x7289DA);

    ctx.send(poise::CreateReply::default().embed(embed)).await?;

    Ok(())
}

// handle for the subtract cmd
pub async fn subtract(
    ctx: poise::Context<'_, Data, Error>,
    target: poise::serenity_prelude::User,
    amount: i64,
) -> Result<(), Error> {
    if !is_admin(&ctx).await {
        ctx.send(poise::CreateReply::default().embed(
            CreateEmbed::new()
                .title("Subtract")
                .description("Only admins can use this command!")
        )).await?;
        return Ok(());
    }
    
    ensure_user(&ctx).await?;
    
    let botav = ctx.cache().current_user().avatar_url().unwrap_or_default();
    
    let target_id = target.id.to_string();
    sqlx::query("INSERT OR IGNORE INTO users (id, bits) VALUES (?, 0)")
        .bind(&target_id)
        .execute(&ctx.data().db)
        .await?;
    
    sqlx::query("UPDATE users SET bits = bits - ? WHERE id = ?")
        .bind(amount)
        .bind(&target_id)
        .execute(&ctx.data().db)
        .await?;
    
    let newbal: i64 = sqlx::query("SELECT bits FROM users WHERE id = ?")
        .bind(&target_id)
        .fetch_one(&ctx.data().db)
        .await?
        .get("bits");
    
    ctx.send(poise::CreateReply::default().embed(
        CreateEmbed::new()
            .author(CreateEmbedAuthor::new(target.name.clone()).icon_url(target.avatar_url().unwrap_or_default()))
            .title("Transaction Complete")
            .description(format!(
                "{} subtracted {} bits from {}\n\n**Balance**\n{} bits", 
                ctx.author().mention(),
                amount,
                target.mention(),
                newbal
            ))
            .footer(CreateEmbedFooter::new("Bytes").icon_url(botav))
            .color(0x5865F2)
    )).await?;
    
    Ok(())
}

// set func for setting balance
pub async fn set(
    ctx: poise::Context<'_, Data, Error>,
    target: poise::serenity_prelude::User,
    amount: u64,
) -> Result<(), Error> {
    if !is_admin(&ctx).await {
        ctx.send(poise::CreateReply::default().embed(
            CreateEmbed::new()
                .title("Set")
                .description("Only admins can use this command. :3")
                .color(0x7289DA),
        )).await?;
        return Ok(());
    }

    let target_id = target.id.to_string();

    sqlx::query("INSERT OR IGNORE INTO users (id, bits) VALUES (?, 0)")
        .bind(&target_id)
        .execute(&ctx.data().db)
        .await?;

    sqlx::query("UPDATE users SET bits = ? WHERE id = ?")
        .bind(amount as i64)
        .bind(&target_id)
        .execute(&ctx.data().db)
        .await?;

    let botuser = ctx.serenity_context().http.get_current_user().await?;

    let embed = CreateEmbed::new()
        .author(
            poise::serenity_prelude::CreateEmbedAuthor::new(
                target.global_name.clone().unwrap_or_else(|| target.name.clone())
            ).icon_url(target.avatar_url().unwrap_or_default())
        )
        .title("Balance Set Successfully")
        .description(format!(
            "<@{}> set <@{}>'s balance to {} bits.",
            ctx.author().id,
            target.id,
            amount
        ))
        .field("Balance", format!("{} bits", amount), false)
        .footer(
            poise::serenity_prelude::CreateEmbedFooter::new("Bytes")
                .icon_url(botuser.avatar_url().unwrap_or_default())
        )
        .color(0x7289DA);

    ctx.send(poise::CreateReply::default().embed(embed)).await?;

    Ok(())
}

// tax setter for admins (per user)
pub async fn tax(
    ctx: poise::Context<'_, Data, Error>,
    target: poise::serenity_prelude::User,
    percent: f64,
) -> Result<(), Error> {
    if !is_admin(&ctx).await {
        ctx.send(poise::CreateReply::default().embed(
            CreateEmbed::new()
                .title("Tax")
                .description("Only admins can use this command. :>")
                .color(0x7289DA),
        )).await?;
        return Ok(());
    }

    if percent < 0.0 || percent > 100.0 {
        ctx.send(poise::CreateReply::default().embed(
            CreateEmbed::new()
                .title("Tax")
                .description("Tax percent must be between 0 and 100. :3")
                .color(0x7289DA),
        )).await?;
        return Ok(());
    }

    let target_id = target.id.to_string();

    sqlx::query("INSERT OR REPLACE INTO user_taxes (user_id, tax_percent) VALUES (?, ?)")
        .bind(&target_id)
        .bind(percent)
        .execute(&ctx.data().db)
        .await?;

    let botuser = ctx.serenity_context().http.get_current_user().await?;
    let guild_id = ctx.guild_id().unwrap();
    let author = guild_id.member(&ctx.serenity_context().http, ctx.author().id).await?;
    let auname = author.nick.clone().unwrap_or_else(|| ctx.author().global_name.clone().unwrap_or(ctx.author().name.clone()));

    let embed = CreateEmbed::new()
        .author(
            poise::serenity_prelude::CreateEmbedAuthor::new(&auname)
                .icon_url(ctx.author().avatar_url().unwrap_or_default())
        )
        .title("Tax Imposed Successfully")
        .description(format!(
            "<@{}> imposed **{:.1}%** daily tax on <@{}>",
            ctx.author().id,
            percent,
            target.id
        ))
        .footer(
            poise::serenity_prelude::CreateEmbedFooter::new("Bytes")
                .icon_url(botuser.avatar_url().unwrap_or_default())
        )
        .color(0x7289DA);

    ctx.send(poise::CreateReply::default().embed(embed)).await?;

    Ok(())
}

// daily counter with irs
pub async fn daily(ctx: poise::Context<'_, Data, Error>) -> Result<(), Error> {
    ensure_user(&ctx).await?;
    let user_id = ctx.author().id.to_string();
    let now = Utc::now().to_rfc3339();

    let last_daily: Option<String> = sqlx::query("SELECT last_daily FROM users WHERE id = ?")
        .bind(&user_id)
        .fetch_one(&ctx.data().db)
        .await?
        .get("last_daily");

    if let Some(lt_str) = last_daily {
        if let Ok(lt) = DateTime::parse_from_rfc3339(&lt_str) {
            let time_diff = Utc::now().signed_duration_since(lt);
            if time_diff.num_hours() < 24 {
                let hrs = 24 - time_diff.num_hours();
                ctx.send(poise::CreateReply::default().embed(
                    CreateEmbed::new()
                        .title("Daily")
                        .description(format!(
                            "You've already claimed your daily today.\nTry again in **{} hour(s)**.",
                            hrs
                        ))
                        .color(0x7289DA),
                )).await?;
                return Ok(());
            }
        }
    }

    let full_rwd = rand::thread_rng().gen_range(50..=150);
    let tax_percent: f64 = sqlx::query("SELECT tax_percent FROM user_taxes WHERE user_id = ?")
        .bind(&user_id)
        .fetch_optional(&ctx.data().db)
        .await?
        .map(|row: sqlx::sqlite::SqliteRow| row.get("tax_percent"))
        .unwrap_or(0.0);

    let tax_amt = ((full_rwd as f64) * (tax_percent / 100.0)).round() as i64;
    let net_rwd = full_rwd - tax_amt;

    sqlx::query("UPDATE users SET bits = bits + ?, last_daily = ? WHERE id = ?")
        .bind(net_rwd)
        .bind(&now)
        .bind(&user_id)
        .execute(&ctx.data().db)
        .await?;

    let botuser = ctx.serenity_context().http.get_current_user().await?;
    let guild_id = ctx.guild_id().unwrap();
    let member = guild_id.member(&ctx.serenity_context().http, ctx.author().id).await?;
    let auname = member.nick.clone().unwrap_or_else(|| ctx.author().global_name.clone().unwrap_or(ctx.author().name.clone()));

    let mut embed = CreateEmbed::new()
        .author(
            poise::serenity_prelude::CreateEmbedAuthor::new(&auname)
                .icon_url(ctx.author().avatar_url().unwrap_or_default())
        )
        .title("Daily Reward");

    embed = embed
        .field("Amount", format!("{} Bits", net_rwd), true)
        .field("Next Reward", "In 1 day", true);

    if tax_percent > 0.0 {
        embed = embed.field("Amount Taxed", format!("{} Bits", tax_amt), true);
    }

    embed = embed.footer(
        poise::serenity_prelude::CreateEmbedFooter::new("Bytes")
            .icon_url(botuser.avatar_url().unwrap_or_default())
    ).color(0x7289DA);

    ctx.send(poise::CreateReply::default().embed(embed)).await?;

    Ok(())
}

// weekly counter with member nickname UI
pub async fn weekly(ctx: poise::Context<'_, Data, Error>) -> Result<(), Error> {
    ensure_user(&ctx).await?;
    let user_id = ctx.author().id.to_string();
    let now = Utc::now().to_rfc3339();

    let last_weekly: Option<String> = sqlx::query("SELECT last_weekly FROM users WHERE id = ?")
        .bind(&user_id)
        .fetch_one(&ctx.data().db)
        .await?
        .get("last_weekly");

    let guild_id = ctx.guild_id().unwrap();
    let member = guild_id.member(&ctx.serenity_context().http, ctx.author().id).await?;
    let nickname = member.nick.clone().unwrap_or_else(|| ctx.author().name.clone());
    let bot_user = ctx.serenity_context().http.get_current_user().await?;

    if let Some(lt_str) = last_weekly {
        if let Ok(lt) = DateTime::parse_from_rfc3339(&lt_str) {
            let time_diff = Utc::now().signed_duration_since(lt);
            if time_diff.num_days() < 7 {
                let embed = CreateEmbed::new()
                    .author(
                        poise::serenity_prelude::CreateEmbedAuthor::new(nickname)
                            .icon_url(ctx.author().avatar_url().unwrap_or_default())
                    )
                    .title("Weekly Reward")
                    .description("You have already claimed your weekly reward!\n\n**Try again in 7 days.**")
                    .footer(
                        poise::serenity_prelude::CreateEmbedFooter::new("Bytes")
                            .icon_url(bot_user.avatar_url().unwrap_or_default())
                    )
                    .color(0x7289DA);

                ctx.send(poise::CreateReply::default().embed(embed)).await?;
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

    let embed = CreateEmbed::new()
        .author(
            poise::serenity_prelude::CreateEmbedAuthor::new(nickname)
                .icon_url(ctx.author().avatar_url().unwrap_or_default())
        )
        .title("Weekly Reward")
        .field("Amount", format!("{} Bits", reward), true)
        .field("Next Reward", "In 7 days", true)
        .footer(
            poise::serenity_prelude::CreateEmbedFooter::new("Bytes")
                .icon_url(bot_user.avatar_url().unwrap_or_default())
        )
        .color(0x7289DA);

    ctx.send(poise::CreateReply::default().embed(embed)).await?;
    Ok(())
}

//handle for monthly
pub async fn monthly(ctx: poise::Context<'_, Data, Error>) -> Result<(), Error> {
    ensure_user(&ctx).await?;
    
    let botav = ctx.cache().current_user().avatar_url().unwrap_or_default();
    let user_id = ctx.author().id.to_string();
    let now = Utc::now().to_rfc3339();
    
    let last_monthly: Option<String> = sqlx::query("SELECT last_monthly FROM users WHERE id = ?")
        .bind(&user_id)
        .fetch_one(&ctx.data().db)
        .await?
        .get("last_monthly");
    
    if let Some(lt_str) = last_monthly {
        if let Ok(lt) = DateTime::parse_from_rfc3339(&lt_str) {
            let time_diff = Utc::now().signed_duration_since(lt);
            if time_diff.num_days() < 30 {
                let days_left = 30 - time_diff.num_days();
                
                ctx.send(poise::CreateReply::default().embed(
                    CreateEmbed::new()
                        .author(CreateEmbedAuthor::new(ctx.author().name.clone()).icon_url(ctx.author().avatar_url().unwrap_or_default()))
                        .title("Monthly Claim")
                        .description(format!("You already claimed your monthly reward!\n\n**Try again in {} days**", days_left))
                        .footer(CreateEmbedFooter::new("bits").icon_url(botav))
                        .color(0x7289DA)
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
    
    let newbal: i64 = sqlx::query("SELECT bits FROM users WHERE id = ?")
        .bind(&user_id)
        .fetch_one(&ctx.data().db)
        .await?
        .get("bits");
    
    ctx.send(poise::CreateReply::default().embed(
        CreateEmbed::new()
            .author(CreateEmbedAuthor::new(ctx.author().name.clone()).icon_url(ctx.author().avatar_url().unwrap_or_default()))
            .title("Monthly Reward Claimed")
            .description(format!(
                "You claimed your monthly reward of **{} bits**!\n\n**Balance**\n{} bits", 
                reward,
                newbal
            ))
            .footer(CreateEmbedFooter::new("bits").icon_url(botav))
            .color(0x7289DA)
    )).await?;
    
    Ok(())
}

//handle for yearly
pub async fn yearly(ctx: poise::Context<'_, Data, Error>) -> Result<(), Error> {
    ensure_user(&ctx).await?;

    let author = ctx.author();
    let user_id = author.id.to_string();
    let now = Utc::now();

    let bot_user = ctx.serenity_context().http.get_current_user().await?;
    let guild_id = ctx.guild_id().unwrap_or_default();

    let server_name = guild_id
        .member(&ctx.serenity_context().http, author.id)
        .await
        .ok()
        .and_then(|m| m.nick)
        .unwrap_or_else(|| author.name.clone());

    let last_yearly: Option<String> = sqlx::query("SELECT last_yearly FROM users WHERE id = ?")
        .bind(&user_id)
        .fetch_one(&ctx.data().db)
        .await?
        .get("last_yearly");

    if let Some(lt_str) = last_yearly {
        if let Ok(last_claim) = DateTime::parse_from_rfc3339(&lt_str) {
            let next_claim = last_claim.with_timezone(&Utc) + chrono::Duration::days(365);
            if now < next_claim {
                let remaining = next_claim - now;
                let days = remaining.num_days();
                let hours = remaining.num_hours() % 24;
                let time_msg = if days > 0 {
                    if hours > 0 {
                        format!("{} days and {} hours", days, hours)
                    } else {
                        format!("{} days", days)
                    }
                } else {
                    format!("{} hours", remaining.num_hours())
                };

                ctx.send(poise::CreateReply::default().embed(
                    CreateEmbed::new()
                        .author(CreateEmbedAuthor::new(server_name.clone())
                            .icon_url(author.avatar_url().unwrap_or_default()))
                        .title("Yearly Claim")
                        .description(format!("You already claimed your yearly reward!\n\n**Try again in {}**", time_msg))
                        .footer(CreateEmbedFooter::new("Bytes")
                            .icon_url(bot_user.avatar_url().unwrap_or_default()))
                        .color(0x7289DA)
                )).await?;
                return Ok(());
            }
        }
    }

    let reward = 25_000;
    let now_rfc = now.to_rfc3339();
    sqlx::query("UPDATE users SET bits = bits + ?, last_yearly = ? WHERE id = ?")
        .bind(reward)
        .bind(&now_rfc)
        .bind(&user_id)
        .execute(&ctx.data().db).await?;

    let newbal: i64 = sqlx::query("SELECT bits FROM users WHERE id = ?")
        .bind(&user_id)
        .fetch_one(&ctx.data().db).await?
        .get("bits");

    ctx.send(poise::CreateReply::default().embed(
        CreateEmbed::new()
            .author(CreateEmbedAuthor::new(server_name)
                .icon_url(author.avatar_url().unwrap_or_default()))
            .title("Yearly Reward Claimed")
            .description(format!(
                "You claimed your yearly reward of **{} Bits**!\n\n**Balance:** {}\n", 
                reward,
                newbal
            ))
            .footer(CreateEmbedFooter::new("Bytes")
                .icon_url(bot_user.avatar_url().unwrap_or_default()))
            .color(0x7289DA)
    )).await?;

    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShopItem {
    pub id: String,
    pub name: String,
    pub desc: String,
    pub price: i64,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserItem {
    pub item_id: String,
    pub quantity: i32,
    pub owned_at: String,
}

pub async fn shop(ctx: poise::Context<'_, Data, Error>) -> Result<(), Error> {
    ensure_user(&ctx).await?;
    
    let botav = ctx.cache().current_user().avatar_url().unwrap_or_default();
    
    let userbal: i64 = sqlx::query("SELECT bits FROM users WHERE id = ?")
        .bind(ctx.author().id.to_string())
        .fetch_one(&ctx.data().db)
        .await?
        .get("bits");
    
    let items_raw = sqlx::query("SELECT id, name, description, price, tags FROM shop_items ORDER BY price ASC LIMIT 5")
        .fetch_all(&ctx.data().db)
        .await?;
    
    let items: Vec<ShopItem> = items_raw.into_iter().map(|row| ShopItem {
        id: row.get("id"),
        name: row.get("name"),
        desc: row.get("description"),
        price: row.get("price"),
        tags: serde_json::from_str(&row.get::<String, _>("tags")).unwrap_or_default(),
    }).collect();

    if items.is_empty() {
        ctx.send(poise::CreateReply::default()
            .embed(
                CreateEmbed::new()
                    .title("Shop Temporarily Closed")
                    .description("No items are currently available. Please check back later!")
                    .color(0xED4245)
            )
        ).await?;
        return Ok(());
    }
    
    let shop_desc = format!(
        "**Your Balance:** {} bits\n\n**Available Items:**",
        userbal
    );
    
    let mut fields = Vec::new();
    let mut buttons = Vec::new();
    
    for (index, item) in items.iter().enumerate() {
        let tags_display = if !item.tags.is_empty() {
            let mut tags_pretty = Vec::new();
            for tag in &item.tags {
                match tag.as_str() {
                    "buy_once" => tags_pretty.push("Buy Once"),
                    _ => tags_pretty.push(tag), // fallback
                }
            }
            format!("\n**Tags:** {}", tags_pretty.join(", "))
        } else {
            String::new()
        };

        fields.push((
            format!("{}", item.name),
            format!("**Price:** {} bits\n*{}*{}", item.price, item.desc, tags_display),
            true
        ));

        if index < 5 {
            buttons.push(
                CreateButton::new(format!("shop_buy_{}", item.id))
                    .label(format!("Buy {} ({})", item.name, item.price))
                    .style(ButtonStyle::Success)
            );
        }
    }
    
    let comps = vec![CreateActionRow::Buttons(buttons)];
    
    ctx.send(poise::CreateReply::default()
        .embed(
            CreateEmbed::new()
                .author(CreateEmbedAuthor::new(
                    format!("{}'s Shop", ctx.author().display_name())
                ).icon_url(ctx.author().avatar_url().unwrap_or_default()))
                .title("Bytes Shop")
                .description(&shop_desc)
                .fields(fields)
                .footer(CreateEmbedFooter::new("Click the buttons below to purchase items")
                    .icon_url(botav))
                .color(0x5865F2) 
                .thumbnail(ctx.author().avatar_url().unwrap_or_default())
        )
        .components(comps)
    ).await?;
    
    Ok(())
}

//private sec for shop
pub async fn shop_back(
    ctx: &poise::serenity_prelude::Context,
    interaction: &poise::serenity_prelude::ComponentInteraction,
    data: &Data,
    item_id: &str,
) -> Result<(), Error> {
    let user_id = interaction.user.id.to_string();
    
    let item_row = sqlx::query("SELECT id, name, description, price, tags FROM shop_items WHERE id = ?")
        .bind(item_id)
        .fetch_optional(&data.db)
        .await?;
    
    let item = match item_row {
        Some(row) => ShopItem {
            id: row.get("id"),
            name: row.get("name"),
            desc: row.get("description"),
            price: row.get("price"),
            tags: serde_json::from_str(&row.get::<String, _>("tags")).unwrap_or_default(),
        },
        None => return Ok(()),
    };
    
    let userbal: i64 = sqlx::query("SELECT bits FROM users WHERE id = ?")
        .bind(&user_id)
        .fetch_one(&data.db)
        .await?
        .get("bits");
    
    let botav = ctx.cache()
        .as_ref()
        .and_then(|cache| cache.current_user().avatar_url())
        .unwrap_or_default();

    if item.tags.contains(&"buy_once".to_string()) {
        let owned: i64 = sqlx::query("SELECT COUNT(*) as count FROM user_it WHERE user_id = ? AND item_id = ?")
            .bind(&user_id)
            .bind(&item.id)
            .fetch_one(&data.db)
            .await?
            .get("count");
        
        if owned > 0 {
            interaction.create_response(&ctx.http, poise::serenity_prelude::CreateInteractionResponse::Message(
                poise::serenity_prelude::CreateInteractionResponseMessage::new()
                    .embed(CreateEmbed::new()
                        .author(CreateEmbedAuthor::new(
                            interaction.user.display_name().to_string()
                        ).icon_url(interaction.user.avatar_url().unwrap_or_default()))
                        .title("Item Already Owned")
                        .description(format!(
                            "You already own **{}**!\n\nThis is a limited item that can only be purchased once.",
                            item.name
                        ))
                        .color(0xED4245)
                        .footer(CreateEmbedFooter::new("Bytes")
                            .icon_url(botav))
                    )
                    .ephemeral(true)
            )).await?;
            return Ok(());
        }
    }

    if userbal < item.price {
        let need = item.price - userbal;
        interaction.create_response(&ctx.http, poise::serenity_prelude::CreateInteractionResponse::Message(
            poise::serenity_prelude::CreateInteractionResponseMessage::new()
                .embed(CreateEmbed::new()
                    .author(CreateEmbedAuthor::new(
                        interaction.user.display_name().to_string()
                    ).icon_url(interaction.user.avatar_url().unwrap_or_default()))
                    .title("Insufficient bits")
                    .description(format!(
                        "You don't have enough bits to buy **{}**!\n\n**Required:** {} bits\n**You have:** {} bits\n**Need:** {} more bits",
                        item.name, item.price, userbal, need
                    ))
                    .color(0xED4245)
                    .footer(CreateEmbedFooter::new("Bytes")
                        .icon_url(botav))
                )
                .ephemeral(true)
        )).await?;
        return Ok(());
    }
    
    sqlx::query("UPDATE users SET bits = bits - ? WHERE id = ?")
        .bind(item.price)
        .bind(&user_id)
        .execute(&data.db)
        .await?;
    
    let owned_at = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
    sqlx::query("INSERT INTO user_it (user_id, item_id, quantity, owned_at) VALUES (?, ?, 1, ?) ON CONFLICT(user_id, item_id) DO UPDATE SET quantity = quantity + 1")
        .bind(&user_id)
        .bind(&item.id)
        .bind(&owned_at)
        .execute(&data.db)
        .await?;
    
    let newbal = userbal - item.price;

    interaction.create_response(&ctx.http, poise::serenity_prelude::CreateInteractionResponse::Message(
        poise::serenity_prelude::CreateInteractionResponseMessage::new()
            .embed(CreateEmbed::new()
                .author(CreateEmbedAuthor::new(
                    interaction.user.display_name().to_string()
                ).icon_url(interaction.user.avatar_url().unwrap_or_default()))
                .title("Purchase Successful!")
                .description(format!(
                    "You successfully purchased **{}**!\n\n**Cost:** {} bits\n**New Balance:** {} bits\n\nThank you for your purchase!",
                    item.name, item.price, newbal
                ))
                .color(0x57F287)
                .footer(CreateEmbedFooter::new("Bytes")
                    .icon_url(botav))
            )
    )).await?;
    
    Ok(())
}

// handle for backpack
pub async fn backpack(
    ctx: poise::Context<'_, Data, Error>,
    user: Option<poise::serenity_prelude::User>,
) -> Result<(), Error> {
    let target = user.as_ref().unwrap_or(ctx.author());
    ensure_user(&ctx).await?;
    
    let botav = ctx.cache().current_user().avatar_url().unwrap_or_default();
    
    let userit_raw = sqlx::query(
        "SELECT ui.item_id, ui.quantity, ui.owned_at, si.name, si.description, si.price, si.tags 
         FROM user_it ui 
         JOIN shop_items si ON ui.item_id = si.id 
         WHERE ui.user_id = ? 
         ORDER BY ui.owned_at DESC"
    )
    .bind(target.id.to_string())
    .fetch_all(&ctx.data().db)
    .await?;
    
    let user_it: Vec<(UserItem, ShopItem)> = userit_raw.into_iter().map(|row| {
        let user_item = UserItem {
            item_id: row.get("item_id"),
            quantity: row.get("quantity"),
            owned_at: row.get("owned_at"),
        };
        let shop_item = ShopItem {
            id: row.get("item_id"),
            name: row.get("name"),
            desc: row.get("description"),
            price: row.get("price"),
            tags: serde_json::from_str(&row.get::<String, _>("tags")).unwrap_or_default(),
        };
        (user_item, shop_item)
    }).collect();

    if user_it.is_empty() {
        let description = if target.id == ctx.author().id {
            "Your backpack is empty! Visit the shop to buy some items."
        } else {
            "This user's backpack is empty."
        };
        
        ctx.send(poise::CreateReply::default()
            .embed(
                CreateEmbed::new()
                    .author(CreateEmbedAuthor::new(
                        format!("{}'s Backpack", target.display_name())
                    ).icon_url(target.avatar_url().unwrap_or_default()))
                    .title("Empty Backpack")
                    .description(description)
                    .color(0x5865F2)
                    .footer(CreateEmbedFooter::new("Bytes")
                        .icon_url(botav))
            )
        ).await?;
        return Ok(());
    }

    let mut fields = Vec::new();
    
    for (user_item, shop_item) in user_it.iter().take(25) {
        let tags_display = if !shop_item.tags.is_empty() {
            format!(" [{}]", shop_item.tags.join(", "))
        } else {
            String::new()
        };
        
        let quantity_text = if user_item.quantity > 1 {
            format!(" x{}", user_item.quantity)
        } else {
            String::new()
        };
        
        fields.push((
            format!("{}{}{}", shop_item.name, tags_display, quantity_text),
            format!("*{}*\n**Value:** {} bits", shop_item.desc, shop_item.price),
            true
        ));
    }
    
    let tt_val: i64 = user_it.iter()
        .map(|(user_item, shop_item)| shop_item.price * user_item.quantity as i64)
        .sum();
    
    let bp_desc = format!(
        "**Total Items:** {}\n**Total Value:** {} bits",
        user_it.iter().map(|(ui, _)| ui.quantity as i64).sum::<i64>(),
        tt_val
    );

    ctx.send(poise::CreateReply::default()
        .embed(
            CreateEmbed::new()
                .author(CreateEmbedAuthor::new(
                    format!("{}'s Backpack", target.display_name())
                ).icon_url(target.avatar_url().unwrap_or_default()))
                .title("Item Collection")
                .description(&bp_desc)
                .fields(fields)
                .footer(CreateEmbedFooter::new("Bytes")
                    .icon_url(botav))
                .color(0x5865F2)
                .thumbnail(target.avatar_url().unwrap_or_default())
        )
    ).await?;
    
    Ok(())
}

//handle for additem 
pub async fn additem(
    ctx: poise::Context<'_, Data, Error>,
    id: String,
    name: String,
    description: String,
    price: i64,
    tags: Option<String>,
) -> Result<(), Error> {
    if !crate::handlers::is_admin(&ctx).await {
        let botav = ctx.cache().current_user().avatar_url().unwrap_or_default();
        ctx.send(poise::CreateReply::default().embed(
            CreateEmbed::new()
                .author(serenity::all::CreateEmbedAuthor::new(ctx.author().name.clone())
                    .icon_url(ctx.author().avatar_url().unwrap_or_default()))
                .title("Add Item")
                .description("Only admins can add items.")
                .color(0xED4245)
                .footer(serenity::all::CreateEmbedFooter::new("Bytes").icon_url(botav)),
        )).await?;
        return Ok(());
    }

    let json_tags = tags
        .as_deref()
        .map(|t| {
            serde_json::to_string(
                &t.split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect::<Vec<_>>(),
            )
            .unwrap_or_else(|_| "[]".to_string())
        })
        .unwrap_or_else(|| "[]".to_string());

    sqlx::query("INSERT INTO shop_items (id, name, description, price, tags) VALUES (?, ?, ?, ?, ?)")
        .bind(&id)
        .bind(&name)
        .bind(&description)
        .bind(price)
        .bind(json_tags)
        .execute(&ctx.data().db)
        .await?;

    let botav = ctx.cache().current_user().avatar_url().unwrap_or_default();
    ctx.send(
        poise::CreateReply::default().embed(
            CreateEmbed::new()
                .author(serenity::all::CreateEmbedAuthor::new(ctx.author().name.clone())
                    .icon_url(ctx.author().avatar_url().unwrap_or_default()))
                .title("Item Added")
                .description(format!("Successfully added **{}** to the shop!", name))
                .field("Item ID", id, true)
                .field("Price", format!("{} bits", price), true)
                .field("Tags", tags.unwrap_or_else(|| "None".to_string()), false)
                .color(0x57F287)
                .footer(serenity::all::CreateEmbedFooter::new("Bytes").icon_url(botav)),
        ),
    ).await?;

    Ok(())
}