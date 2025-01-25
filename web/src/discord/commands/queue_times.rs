use super::utils::{autocomplete_world, create_queue_embed};
use super::Context;
use super::Error;
use crate::{
    db,
    discord::travel_param::{get_travel_params, TravelDatacenterParam},
};
use poise::CreateReply;

#[poise::command(slash_command, rename = "queue", subcommands("datacenter", "world"))]
#[allow(clippy::unused_async)]
pub async fn queue_times(_: Context<'_>) -> Result<(), Error> {
    Ok(())
}

/// Check queue times for a datacenter
#[poise::command(
    slash_command,
    install_context = "Guild|User",
    interaction_context = "Guild|BotDm|PrivateChannel"
)]
async fn datacenter(
    ctx: Context<'_>,
    #[description = "Datacenter to check for"] datacenter: TravelDatacenterParam,
) -> Result<(), Error> {
    let client = ctx.data();
    let db = client.db();
    let estimates = db::get_queue_estimates_by_datacenter_id(db, vec![datacenter.id]).await?;
    let travel_data = get_travel_params().ok_or(Error::UnknownWorld)?;
    let datacenter = travel_data
        .get_datacenter_by_id(datacenter.id)
        .ok_or(Error::UnknownDatacenter)?;
    let worlds = estimates
        .into_iter()
        .map(|estimate| {
            travel_data
                .get_world_by_id(estimate.world_id)
                .map(|v| (v, estimate))
                .ok_or(Error::UnknownWorld)
        })
        .collect::<Result<Vec<_>, _>>()?;

    let embed = create_queue_embed(&datacenter.name(), worlds);

    ctx.send(CreateReply::default().reply(true).embed(embed))
        .await?;

    Ok(())
}

/// Check queue times for a world
#[poise::command(slash_command)]
async fn world(
    ctx: Context<'_>,
    #[description = "World to check for"]
    #[autocomplete = "autocomplete_world"]
    world: u16,
) -> Result<(), Error> {
    let world = get_travel_params()
        .and_then(|v| v.get_world_by_id(world))
        .cloned()
        .ok_or(Error::UnknownWorld)?;

    let client = ctx.data();
    let db = client.db();
    let estimate = db::get_queue_estimates_by_world_id(db, vec![world.id])
        .await?
        .pop()
        .ok_or(Error::UnknownWorld)?;

    let embed = create_queue_embed(&world.name(), vec![(&world, estimate)]);

    ctx.send(CreateReply::default().reply(true).embed(embed))
        .await?;

    Ok(())
}
