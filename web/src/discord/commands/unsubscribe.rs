use super::utils::autocomplete_world;
use super::Context;
use super::Error;
use crate::{
    discord::utils::{COLOR_ERROR, COLOR_SUCCESS},
    storage::game::worlds::{self, Datacenter},
    subscriptions::{Endpoint, Subscriber},
};
use ::serenity::all::CreateEmbed;
use poise::CreateReply;

#[poise::command(
    slash_command,
    install_context = "Guild|User",
    interaction_context = "Guild|BotDm|PrivateChannel",
    rename = "remindoff",
    subcommands("datacenter", "world")
)]
#[allow(clippy::unused_async)]
pub async fn unsubscribe(_: Context<'_>) -> Result<(), Error> {
    Ok(())
}

/// Remove a DC travel reminder for a datacenter (opposite of /remind)
#[poise::command(slash_command)]
async fn datacenter(
    ctx: Context<'_>,
    #[description = "Datacenter to remind for"] datacenter: Datacenter,
) -> Result<(), Error> {
    let client = ctx.data();
    let subscriptions = client.subscriptions();

    let success = subscriptions
        .unsubscribe(
            Endpoint::Datacenter(datacenter.id),
            &Subscriber::Discord(ctx.author().id.get()),
        )
        .await?;
    let embed = if success {
        CreateEmbed::new()
            .title(format!("Unsubscribed from {}", datacenter))
            .description("You will no longer be reminded when this datacenter is open for travel.")
            .color(COLOR_SUCCESS)
    } else {
        CreateEmbed::new()
            .title(format!("No reminder for {}", datacenter))
            .description("You don't have any reminders for this datacenter.")
            .color(COLOR_ERROR)
    };

    ctx.send(CreateReply::default().reply(true).embed(embed))
        .await?;
    Ok(())
}

/// Remove a DC travel reminder for a specific world (opposite of /remind)
#[poise::command(slash_command)]
async fn world(
    ctx: Context<'_>,
    #[description = "World to remind for"]
    #[autocomplete = "autocomplete_world"]
    world: u16,
) -> Result<(), Error> {
    let world = worlds::get_data()
        .get_world_by_id(world)
        .cloned()
        .ok_or(Error::UnknownWorld)?;

    let client = ctx.data();
    let subscriptions = client.subscriptions();

    let success = subscriptions
        .unsubscribe(
            Endpoint::World(world.id),
            &Subscriber::Discord(ctx.author().id.get()),
        )
        .await?;
    let embed = if success {
        CreateEmbed::new()
            .title(format!("Unsubscribed from {}", world))
            .description("You will no longer be reminded when this world is open for travel.")
            .color(COLOR_SUCCESS)
    } else {
        CreateEmbed::new()
            .title(format!("No reminder for {}", world))
            .description("You don't have any reminders for this world.")
            .color(COLOR_ERROR)
    };

    ctx.send(CreateReply::default().reply(true).embed(embed))
        .await?;

    Ok(())
}
