use super::utils::autocomplete_world;
use super::Context;
use super::Error;
use crate::{
    discord::{
        travel_param::{get_travel_params, TravelDatacenterParam, TravelWorldParam},
        utils::{COLOR_ERROR, COLOR_SUCCESS},
    },
    subscriptions::{Endpoint, Subscriber},
};
use ::serenity::all::CreateEmbed;
use poise::CreateReply;

/// Remove a datacenter travel reminder (opposite of /remind)
#[poise::command(slash_command, rename = "remindoff")]
pub async fn unsubscribe(
    ctx: Context<'_>,
    #[description = "Datacenter to remind for"] datacenter: Option<TravelDatacenterParam>,
    #[description = "World to remind for"]
    #[autocomplete = "autocomplete_world"]
    world: Option<u16>,
) -> Result<(), Error> {
    match (datacenter, world) {
        (Some(dc), _) => unsubscribe_dc(ctx, dc).await,
        (None, Some(world)) => {
            unsubscribe_world(
                ctx,
                get_travel_params()
                    .and_then(|v| v.get_world_by_id(world))
                    .cloned()
                    .ok_or(Error::UnknownWorld)?,
            )
            .await
        }
        (None, None) => Err(Error::NoDestination),
    }
}

async fn unsubscribe_dc(ctx: Context<'_>, datacenter: TravelDatacenterParam) -> Result<(), Error> {
    let client = ctx.data();
    let subscriptions = client.subscriptions();

    let success = subscriptions.unsubscribe(
        Endpoint::Datacenter(datacenter.id),
        &Subscriber::Discord(ctx.author().id),
    );
    let embed = if success {
        CreateEmbed::new()
            .title(format!("Unsubscribed from {}", datacenter.name()))
            .description("You will no longer be reminded when this datacenter is open for travel.")
            .color(COLOR_SUCCESS)
    } else {
        CreateEmbed::new()
            .title(format!("No reminder for {}", datacenter.name()))
            .description("You don't have any reminders for this datacenter.")
            .color(COLOR_ERROR)
    };

    ctx.send(CreateReply::default().reply(true).embed(embed))
        .await?;
    Ok(())
}

async fn unsubscribe_world(ctx: Context<'_>, world: TravelWorldParam) -> Result<(), Error> {
    let client = ctx.data();
    let subscriptions = client.subscriptions();

    let success = subscriptions.unsubscribe(
        Endpoint::World(world.id),
        &Subscriber::Discord(ctx.author().id),
    );
    let embed = if success {
        CreateEmbed::new()
            .title(format!("Unsubscribed from {}", world.name()))
            .description("You will no longer be reminded when this world is open for travel.")
            .color(COLOR_SUCCESS)
    } else {
        CreateEmbed::new()
            .title(format!("No reminder for {}", world.name()))
            .description("You don't have any reminders for this world.")
            .color(COLOR_ERROR)
    };

    ctx.send(CreateReply::default().reply(true).embed(embed))
        .await?;

    Ok(())
}
