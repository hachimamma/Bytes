use crate::{Data, Error};
use sqlx::Row;
use poise::serenity_prelude::CreateEmbed;

#[poise::command(slash_command)]
pub async fn daily(ctx: poise::Context<'_, Data, Error>) -> Result<(), Error> {
    crate::handlers::handle_daily(ctx).await
}

#[poise::command(slash_command)]
pub async fn work(ctx: poise::Context<'_, Data, Error>) -> Result<(), Error> {
    crate::handlers::handle_work(ctx).await
}

#[poise::command(slash_command)]
pub async fn balance(ctx: poise::Context<'_, Data, Error>) -> Result<(), Error> {
    crate::handlers::handle_balance(ctx).await
}

#[poise::command(slash_command)]
pub async fn leaderboard(ctx: poise::Context<'_, Data, Error>) -> Result<(), Error> {
    crate::handlers::handle_leaderboard(ctx).await
}

#[poise::command(slash_command)]
pub async fn rob(
    ctx: poise::Context<'_, Data, Error>,
    #[description = "User to rob"] target: poise::serenity_prelude::User
) -> Result<(), Error> {
    crate::handlers::handle_rob(ctx, target).await
}

#[poise::command(slash_command)]
pub async fn bitflip(
    ctx: poise::Context<'_, Data, Error>,
    #[description = "Amount to bet"] bet_amount: i64,
    #[description = "Heads or tails"] bet_side: String
) -> Result<(), Error> {
    crate::handlers::handle_bitflip(ctx, bet_amount, bet_side).await
}

#[poise::command(slash_command)]
pub async fn dice(
    ctx: poise::Context<'_, Data, Error>,
    #[description = "Amount to bet"] bet_amount: i64,
    #[description = "Number to bet on (1-6)"] bet_side: i32
) -> Result<(), Error> {
    crate::handlers::handle_dice(ctx, bet_amount, bet_side).await
}

#[poise::command(slash_command)]
pub async fn pay(
    ctx: poise::Context<'_, Data, Error>,
    #[description = "User to send bits to"] recipient: poise::serenity_prelude::User,
    #[description = "Amount to send"] amount: i64
) -> Result<(), Error> {
    if amount <= 0 {
        ctx.send(poise::CreateReply::default().embed(
            CreateEmbed::new().title("Pay").description("Amount must be positive!")
        )).await?;
        return Ok(());
    }

    let sender_id = ctx.author().id.to_string();
    let recipient_id = recipient.id.to_string();
    
    // Ensure both users exist
    sqlx::query("INSERT OR IGNORE INTO users (id, bits) VALUES (?, 0)")
        .bind(&sender_id)
        .execute(&ctx.data().db)
        .await?;
    sqlx::query("INSERT OR IGNORE INTO users (id, bits) VALUES (?, 0)")
        .bind(&recipient_id)
        .execute(&ctx.data().db)
        .await?;
    
    // Check sender has enough bits
    let sender_bits: i64 = sqlx::query("SELECT bits FROM users WHERE id = ?")
        .bind(&sender_id)
        .fetch_one(&ctx.data().db)
        .await?
        .get("bits");
    
    if sender_bits < amount {
        ctx.send(poise::CreateReply::default().embed(
            CreateEmbed::new().title("Pay").description("You don't have enough Bits!")
        )).await?;
        return Ok(());
    }
    
    // Transfer bits
    sqlx::query("UPDATE users SET bits = bits - ? WHERE id = ?")
        .bind(amount)
        .bind(&sender_id)
        .execute(&ctx.data().db).await?;
    sqlx::query("UPDATE users SET bits = bits + ? WHERE id = ?")
        .bind(amount)
        .bind(&recipient_id)
        .execute(&ctx.data().db).await?;

    ctx.send(poise::CreateReply::default().embed(
        CreateEmbed::new().title("Pay").description(&format!("You sent {} Bits to {}!", amount, recipient.name))
    )).await?;
    Ok(())
}

#[poise::command(slash_command)]
pub async fn monthly(ctx: poise::Context<'_, Data, Error>) -> Result<(), Error> {
    crate::handlers::handle_monthly(ctx).await
}

#[poise::command(slash_command)]
pub async fn weekly(ctx: poise::Context<'_, Data, Error>) -> Result<(), Error> {
    crate::handlers::handle_weekly(ctx).await
}

#[poise::command(slash_command)]
pub async fn yearly(ctx: poise::Context<'_, Data, Error>) -> Result<(), Error> {
    crate::handlers::handle_yearly(ctx).await
}

#[poise::command(slash_command)]
pub async fn add(
    ctx: poise::Context<'_, Data, Error>,
    #[description = "User to credit"] target: poise::serenity_prelude::User,
    #[description = "Bits to add"] amount: i64
) -> Result<(), Error> {
    // Check if user is mod
    let is_mod = if let Some(_guild_id) = ctx.guild_id() {
        if let Some(member) = ctx.author_member().await.as_ref() {
            member.permissions.unwrap_or_default().administrator()
        } else { false }
    } else { false };
    
    if !is_mod {
        ctx.send(poise::CreateReply::default().embed(
            CreateEmbed::new().title("Add").description("Only admins can use this command!")
        )).await?;
        return Ok(());
    }
    
    let target_id = target.id.to_string();
    sqlx::query("INSERT OR IGNORE INTO users (id, bits) VALUES (?, 0)")
        .bind(&target_id)
        .execute(&ctx.data().db)
        .await?;
    
    sqlx::query("UPDATE users SET bits = bits + ? WHERE id = ?")
        .bind(amount)
        .bind(&target_id)
        .execute(&ctx.data().db).await?;
    
    ctx.send(poise::CreateReply::default().embed(
        CreateEmbed::new().title("Add").description(&format!("Added {} Bits to {}!", amount, target.name))
    )).await?;
    Ok(())
}

#[poise::command(slash_command)]
pub async fn subtract(
    ctx: poise::Context<'_, Data, Error>,
    #[description = "User to subtract from"] target: poise::serenity_prelude::User,
    #[description = "Bits to remove"] amount: i64
) -> Result<(), Error> {
    // Check if user is mod
    let is_mod = if let Some(_guild_id) = ctx.guild_id() {
        if let Some(member) = ctx.author_member().await.as_ref() {
            member.permissions.unwrap_or_default().administrator()
        } else { false }
    } else { false };
    
    if !is_mod {
        ctx.send(poise::CreateReply::default().embed(
            CreateEmbed::new().title("Subtract").description("Only admins can use this command!")
        )).await?;
        return Ok(());
    }
    
    let target_id = target.id.to_string();
    sqlx::query("INSERT OR IGNORE INTO users (id, bits) VALUES (?, 0)")
        .bind(&target_id)
        .execute(&ctx.data().db)
        .await?;
    
    sqlx::query("UPDATE users SET bits = bits - ? WHERE id = ?")
        .bind(amount)
        .bind(&target_id)
        .execute(&ctx.data().db).await?;
    
    ctx.send(poise::CreateReply::default().embed(
        CreateEmbed::new().title("Subtract").description(&format!("Subtracted {} Bits from {}!", amount, target.name))
    )).await?;
    Ok(())
}

#[poise::command(slash_command)]
pub async fn set(
    ctx: poise::Context<'_, Data, Error>,
    #[description = "User to set balance"] target: poise::serenity_prelude::User,
    #[description = "New balance"] amount: i64
) -> Result<(), Error> {
    // Check if user is mod
    let is_mod = if let Some(_guild_id) = ctx.guild_id() {
        if let Some(member) = ctx.author_member().await.as_ref() {
            member.permissions.unwrap_or_default().administrator()
        } else { false }
    } else { false };
    
    if !is_mod {
        ctx.send(poise::CreateReply::default().embed(
            CreateEmbed::new().title("Set").description("Only admins can use this command!")
        )).await?;
        return Ok(());
    }
    
    let target_id = target.id.to_string();
    sqlx::query("INSERT OR IGNORE INTO users (id, bits) VALUES (?, 0)")
        .bind(&target_id)
        .execute(&ctx.data().db)
        .await?;
    
    sqlx::query("UPDATE users SET bits = ? WHERE id = ?")
        .bind(amount)
        .bind(&target_id)
        .execute(&ctx.data().db).await?;
    
    ctx.send(poise::CreateReply::default().embed(
        CreateEmbed::new().title("Set").description(&format!("Set {}'s balance to {} Bits!", target.name, amount))
    )).await?;
    Ok(())
}

#[poise::command(slash_command)]
pub async fn tax(
    ctx: poise::Context<'_, Data, Error>,
    #[description = "User to tax"] target: poise::serenity_prelude::User,
    #[description = "Tax percentage (0-100)"] percentage: f64
) -> Result<(), Error> {
    crate::handlers::handle_tax(ctx, target, percentage).await
}