use super::utils::{autocomplete_world, create_travel_embed};
use super::Context;
use super::Error;
use crate::{
    db,
    discord::{
        travel_param::{get_travel_params, TravelDatacenterParam, TravelWorldParam},
        utils::{COLOR_ERROR, COLOR_SUCCESS},
    },
    subscriptions::{Endpoint, Subscriber},
};
use ::serenity::all::CreateEmbed;
use poise::CreateReply;

#[poise::command(slash_command, rename = "remind", subcommands("datacenter", "world"))]
pub async fn subscribe(_: Context<'_>) -> Result<(), Error> {
    Ok(())
}

/// Send a reminder when DC travel is available to a datacenter
#[poise::command(slash_command)]
async fn datacenter(
    ctx: Context<'_>,
    #[description = "Datacenter to remind for"] datacenter: TravelDatacenterParam,
) -> Result<(), Error> {
    subscribe_datacenter(ctx, datacenter).await
}

pub async fn subscribe_datacenter(
    ctx: Context<'_>,
    datacenter: TravelDatacenterParam,
) -> Result<(), Error> {
    let client = ctx.data();
    let db = client.db();
    let config = client.config();
    let subscriptions = client.subscriptions();
    let status = db::get_travel_states_by_datacenter_id(db, vec![datacenter.id]).await?;
    let response = if status.iter().any(|(_, status)| !*status) {
        let travel_data = get_travel_params().ok_or(Error::UnknownWorld)?;
        let datacenter = travel_data
            .get_datacenter_by_id(datacenter.id)
            .ok_or(Error::UnknownDatacenter)?;
        let worlds = status
            .into_iter()
            .map(|(world_id, status)| {
                travel_data
                    .get_world_by_id(world_id)
                    .map(|v| (v, status))
                    .ok_or(Error::UnknownWorld)
            })
            .collect::<Result<Vec<_>, _>>()?;

        create_travel_embed(&datacenter.name(), worlds, config)
            .description("This datacenter is aleady open for travel.")
            .color(COLOR_ERROR)
    } else {
        let success = subscriptions
            .subscribe(
                Endpoint::Datacenter(datacenter.id),
                Subscriber::Discord(ctx.author().id.get()),
            )
            .await?;

        if success {
            CreateEmbed::new()
                .title(format!("Subscribed to {}", datacenter.name()))
                .description("You will be reminded when this datacenter is open for travel.")
                .color(COLOR_SUCCESS)
        } else {
            CreateEmbed::new()
                .title(format!("Already subscribed to {}", datacenter.name()))
                .description("You are already subscribed to this datacenter.")
                .color(COLOR_ERROR)
        }
    };
    ctx.send(CreateReply::default().reply(true).embed(response))
        .await?;
    Ok(())
}

/// Send a reminder when DC travel is available to a world
#[poise::command(slash_command)]
async fn world(
    ctx: Context<'_>,
    #[description = "World to remind for"]
    #[autocomplete = "autocomplete_world"]
    world: u16,
) -> Result<(), Error> {
    let world = get_travel_params()
        .and_then(|v| v.get_world_by_id(world))
        .cloned()
        .ok_or(Error::UnknownWorld)?;
    subscribe_world(ctx, world).await
}

pub async fn subscribe_world(ctx: Context<'_>, world: TravelWorldParam) -> Result<(), Error> {
    let client = ctx.data();
    let db = client.db();
    let config = client.config();
    let subscriptions = client.subscriptions();
    let is_prohibited = db::get_travel_states_by_world_id(db, vec![world.id])
        .await?
        .get(&world.id)
        .copied()
        .unwrap_or_default();
    let response = if !is_prohibited {
        create_travel_embed(&world.name(), vec![(&world, is_prohibited)], config)
            .description("This world is aleady open for travel.")
            .color(COLOR_ERROR)
    } else {
        let success = subscriptions
            .subscribe(
                Endpoint::World(world.id),
                Subscriber::Discord(ctx.author().id.get()),
            )
            .await?;

        if success {
            CreateEmbed::new()
                .title(format!("Subscribed to {}", world.name()))
                .description("You will be reminded when this world is open for travel.")
                .color(COLOR_SUCCESS)
        } else {
            CreateEmbed::new()
                .title(format!("Already subscribed to {}", world.name()))
                .description("You are already subscribed to this world.")
                .color(COLOR_ERROR)
        }
    };
    ctx.send(CreateReply::default().reply(true).embed(response))
        .await?;

    Ok(())
}
