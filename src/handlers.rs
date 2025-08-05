//ignore warnings this works just fine

use crate::{Data, Error};
use chrono::{DateTime, Utc};
use poise::serenity_prelude::CreateEmbed;
use poise::serenity_prelude::{CreateEmbedAuthor, CreateEmbedFooter, CreateActionRow, CreateButton, ButtonStyle};
use num_format::{Locale, ToFormattedString};
use rand::Rng;
use serenity::all::Mentionable;
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
    
    let components = vec![CreateActionRow::Buttons(vec![
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
        .components(components)
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
                        .footer(CreateEmbedFooter::new("Points").icon_url(botav))
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
    
    let new_balance: i64 = sqlx::query("SELECT bits FROM users WHERE id = ?")
        .bind(&user_id)
        .fetch_one(&ctx.data().db)
        .await?
        .get("bits");
    
    ctx.send(poise::CreateReply::default().embed(
        CreateEmbed::new()
            .author(CreateEmbedAuthor::new(ctx.author().name.clone()).icon_url(ctx.author().avatar_url().unwrap_or_default()))
            .title("Monthly Reward Claimed")
            .description(format!(
                "You claimed your monthly reward of **{} points**!\n\n**Balance**\n{} points", 
                reward,
                new_balance
            ))
            .footer(CreateEmbedFooter::new("Points").icon_url(botav))
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

    let new_balance: i64 = sqlx::query("SELECT bits FROM users WHERE id = ?")
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
                new_balance
            ))
            .footer(CreateEmbedFooter::new("Bytes")
                .icon_url(bot_user.avatar_url().unwrap_or_default()))
            .color(0x7289DA)
    )).await?;

    Ok(())
}
