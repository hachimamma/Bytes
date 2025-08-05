use crate::{Data, Error};
use poise::serenity_prelude::{
    CreateEmbed, CreateSelectMenu, CreateSelectMenuKind, CreateSelectMenuOption, 
    CreateActionRow, ComponentInteractionCollector, ReactionType
};
use std::time::Duration;

#[poise::command(slash_command)]
pub async fn daily(ctx: poise::Context<'_, Data, Error>) -> Result<(), Error> {
    crate::handlers::daily(ctx).await
}

#[poise::command(slash_command)]
pub async fn balance(ctx: poise::Context<'_, Data, Error>) -> Result<(), Error> {
    crate::handlers::balance(ctx).await
}

#[poise::command(slash_command)]
pub async fn leaderboard(ctx: poise::Context<'_, Data, Error>) -> Result<(), Error> {
    crate::handlers::leaderboard(ctx).await
}

#[poise::command(slash_command)]
pub async fn rob(
    ctx: poise::Context<'_, Data, Error>,
    #[description = "User to rob"] target: poise::serenity_prelude::User
) -> Result<(), Error> {
    crate::handlers::rob(ctx, target).await
}

#[poise::command(slash_command)]
pub async fn coinflip(
    ctx: poise::Context<'_, Data, Error>,
    #[description = "Amount to bet"] betamt: i64,
) -> Result<(), Error> {
    let embed = CreateEmbed::new()
        .title("Coinflip")
        .description(&format!("**Bet Amount:** {} Bits\n\nSelect your side:", betamt))
        .color(0x00ff00);

    let select_menu = CreateSelectMenu::new(
        "bside",
        CreateSelectMenuKind::String {
            options: vec![
                CreateSelectMenuOption::new("Heads", "heads")
                    .description("Bet on heads")
                    .emoji(ReactionType::Unicode("🌟".to_string())),
                CreateSelectMenuOption::new("Tails", "tails")
                    .description("Bet on tails")
                    .emoji(ReactionType::Unicode("⭐".to_string())),
            ],
        },
    )
    .placeholder("Choose heads or tails :3.");

    let components = vec![CreateActionRow::SelectMenu(select_menu)];

    let reply = poise::CreateReply::default()
        .embed(embed)
        .components(components)
        .ephemeral(true);

    let msg = ctx.send(reply).await?;

    let interaction = ComponentInteractionCollector::new(ctx)
        .timeout(Duration::from_secs(60))
        .filter(move |i| i.data.custom_id == "bside")
        .await;

    if let Some(interaction) = interaction {
        let bet_side = if let poise::serenity_prelude::ComponentInteractionDataKind::StringSelect { values } = &interaction.data.kind {
            values.first().unwrap().clone()
        } else {
            return Ok(());
        };
        
        interaction.create_response(
            &ctx.serenity_context().http,
            poise::serenity_prelude::CreateInteractionResponse::Acknowledge,
        ).await?;

        coinflip_game(ctx, betamt, bet_side, &msg).await?;
    } else {
        let timeout_embed = CreateEmbed::new()
            .title("Coinflip")
            .description("Selection timed out!")
            .color(0xff0000);

        msg.edit(ctx, poise::CreateReply::default()
            .embed(timeout_embed)
            .components(vec![])
        ).await?;
    }

    Ok(())
}

#[poise::command(slash_command)]
pub async fn dice(
    ctx: poise::Context<'_, Data, Error>,
    #[description = "Amount to bet"] betamt: i64,
) -> Result<(), Error> {
    let embed = CreateEmbed::new()
        .title("Dice Roll")
        .description(&format!("**Bet Amount:** {} Bits\n\nSelect your number:", betamt))
        .color(0x0099ff);

    let select_menu = CreateSelectMenu::new(
        "dice_side",
        CreateSelectMenuKind::String {
            options: vec![
                CreateSelectMenuOption::new("1", "1").description("Bet on 1").emoji(ReactionType::Unicode("1️⃣".to_string())),
                CreateSelectMenuOption::new("2", "2").description("Bet on 2").emoji(ReactionType::Unicode("2️⃣".to_string())),
                CreateSelectMenuOption::new("3", "3").description("Bet on 3").emoji(ReactionType::Unicode("3️⃣".to_string())),
                CreateSelectMenuOption::new("4", "4").description("Bet on 4").emoji(ReactionType::Unicode("4️⃣".to_string())),
                CreateSelectMenuOption::new("5", "5").description("Bet on 5").emoji(ReactionType::Unicode("5️⃣".to_string())),
                CreateSelectMenuOption::new("6", "6").description("Bet on 6").emoji(ReactionType::Unicode("6️⃣".to_string())),
            ],
        },
    )
    .placeholder("Choose a number (1-6)");

    let components = vec![CreateActionRow::SelectMenu(select_menu)];

    let reply = poise::CreateReply::default()
        .embed(embed)
        .components(components)
        .ephemeral(true);

    let msg = ctx.send(reply).await?;

    let interaction = ComponentInteractionCollector::new(ctx)
        .timeout(Duration::from_secs(60))
        .filter(move |i| i.data.custom_id == "dice_side")
        .await;

    if let Some(interaction) = interaction {
        let bet_side_str = if let poise::serenity_prelude::ComponentInteractionDataKind::StringSelect { values } = &interaction.data.kind {
            values.first().unwrap().clone()
        } else {
            return Ok(());
        };
        let bet_side: i32 = bet_side_str.parse().unwrap_or(1);
        
        interaction.create_response(
            &ctx.serenity_context().http,
            poise::serenity_prelude::CreateInteractionResponse::Acknowledge,
        ).await?;

        dice_game(ctx, betamt, bet_side, &msg).await?;
    } else {
        let timeout_embed = CreateEmbed::new()
            .title("Dice Roll")
            .description("Selection timed out!")
            .color(0xff0000);

        msg.edit(ctx, poise::CreateReply::default()
            .embed(timeout_embed)
            .components(vec![])
        ).await?;
    }

    Ok(())
}

//helper func for ingame logic
async fn coinflip_game(
    ctx: poise::Context<'_, Data, Error>,
    betamt: i64,
    bet_side: String,
    msg: &poise::ReplyHandle<'_>,
) -> Result<(), Error> {
    let processing_embed = CreateEmbed::new()
        .title("Coinflip")
        .description(&format!("**Bet Amount:** {} Bits\n**Your Choice:** {}\n\nFlipping...", betamt, bet_side))
        .color(0xffff00);

    msg.edit(ctx, poise::CreateReply::default()
        .embed(processing_embed)
        .components(vec![])
    ).await?;

    tokio::time::sleep(Duration::from_millis(1500)).await;

    crate::handlers::coinflip(ctx, betamt, bet_side).await?;

    Ok(())
}

//helper func for post-process
async fn dice_game(
    ctx: poise::Context<'_, Data, Error>,
    betamt: i64,
    bet_side: i32,
    msg: &poise::ReplyHandle<'_>,
) -> Result<(), Error> {
    let processing_embed = CreateEmbed::new()
        .title("Dice Roll")
        .description(&format!("**Bet Amount:** {} Bits\n**Your Number:** {}\n\nRolling...", betamt, bet_side))
        .color(0xffff00);

    msg.edit(ctx, poise::CreateReply::default()
        .embed(processing_embed)
        .components(vec![])
    ).await?;

    tokio::time::sleep(Duration::from_millis(1500)).await;

    crate::handlers::dice(ctx, betamt, bet_side).await?;

    Ok(())
}

#[poise::command(slash_command)]
pub async fn pay(
    ctx: poise::Context<'_, Data, Error>,
    #[description = "User to send bits to"] recipient: poise::serenity_prelude::User,
    #[description = "Amount to send"] amt: i64
) -> Result<(), Error> {
    crate::handlers::pay(ctx, recipient, amt).await
}

#[poise::command(slash_command)]
pub async fn monthly(ctx: poise::Context<'_, Data, Error>) -> Result<(), Error> {
    crate::handlers::monthly(ctx).await
}

#[poise::command(slash_command)]
pub async fn weekly(ctx: poise::Context<'_, Data, Error>) -> Result<(), Error> {
    crate::handlers::weekly(ctx).await
}

#[poise::command(slash_command)]
pub async fn yearly(ctx: poise::Context<'_, Data, Error>) -> Result<(), Error> {
    crate::handlers::yearly(ctx).await
}

#[poise::command(slash_command)]
pub async fn add(
    ctx: poise::Context<'_, Data, Error>,
    #[description = "User to credit"] target: serenity::all::User,
    #[description = "Bits to add"] amt: i64,
) -> Result<(), Error> {
    crate::handlers::add(ctx, target, amt.try_into().unwrap()).await
}

#[poise::command(slash_command)]
pub async fn subtract(
    ctx: poise::Context<'_, Data, Error>,
    #[description = "User to subtract from"] target: poise::serenity_prelude::User,
    #[description = "Bits to remove"] amt: i64
) -> Result<(), Error> {
    crate::handlers::subtract(ctx, target, amt).await
}

#[poise::command(slash_command)]
pub async fn set(
    ctx: poise::Context<'_, Data, Error>,
    #[description = "User to set balance"] target: poise::serenity_prelude::User,
    #[description = "New balance"] amt: i64
) -> Result<(), Error> {
    crate::handlers::set(ctx, target, amt.try_into().unwrap()).await
}

#[poise::command(slash_command)]
pub async fn tax(
    ctx: poise::Context<'_, Data, Error>,
    #[description = "User to tax"] target: poise::serenity_prelude::User,
    #[description = "Tax percentage (0-100)"] percentage: f64
) -> Result<(), Error> {
    crate::handlers::tax(ctx, target, percentage).await
}