use super::Context;
use super::Error;
use super::{
    subscribe::{subscribe_datacenter, subscribe_world},
    utils::{autocomplete_world, create_travel_embed},
};
use crate::storage::{
    db,
    game::worlds::{self, Datacenter},
};
use ::serenity::all::{EditMessage, ReactionType};
use poise::{serenity_prelude as serenity, CreateReply};

#[poise::command(
    slash_command,
    install_context = "Guild|User",
    interaction_context = "Guild|BotDm|PrivateChannel",
    subcommands("datacenter", "world")
)]
#[allow(clippy::unused_async)]
pub async fn travel(_: Context<'_>) -> Result<(), Error> {
    Ok(())
}

/// Check DC travel status for a datacenter
#[poise::command(slash_command)]
async fn datacenter(
    ctx: Context<'_>,
    #[description = "Datacenter to check for"] datacenter: Datacenter,
) -> Result<(), Error> {
    let client = ctx.data();
    let db = client.db();
    let config = client.config();
    let status = db::travel::get_travel_states_by_datacenter_id(db, vec![datacenter.id]).await?;
    let travel_data = worlds::get_data();
    let datacenter = travel_data
        .get_datacenter_by_id(datacenter.id)
        .cloned()
        .ok_or(Error::UnknownDatacenter)?;
    let is_all_prohibited = status.iter().all(|(_, status)| *status);
    let worlds = status
        .into_iter()
        .map(|(world_id, status)| {
            travel_data
                .get_world_by_id(world_id)
                .map(|v| (v, status))
                .ok_or(Error::UnknownWorld)
        })
        .collect::<Result<Vec<_>, _>>()?;

    let embed = create_travel_embed(&datacenter.to_string(), worlds, config);

    let components = if is_all_prohibited {
        vec![serenity::CreateActionRow::Buttons(vec![
            serenity::CreateButton::new("set_reminder")
                .label("Remind Me")
                .emoji(ReactionType::Unicode("⏰".to_owned()))
                .style(poise::serenity_prelude::ButtonStyle::Primary),
        ])]
    } else {
        vec![]
    };

    let reply = ctx
        .send(
            CreateReply::default()
                .reply(true)
                .embed(embed)
                .components(components),
        )
        .await?;

    if is_all_prohibited {
        if let Some(interaction) = reply
            .message()
            .await?
            .await_component_interaction(ctx)
            .author_id(ctx.author().id)
            .timeout(std::time::Duration::from_secs(120))
            .await
        {
            if interaction.data.custom_id == "set_reminder" {
                subscribe_datacenter(ctx, datacenter, true).await?;
            }
            interaction
                .create_response(ctx, serenity::CreateInteractionResponse::Acknowledge)
                .await?;
        }
        let mut msg = reply.into_message().await?;
        if !msg
            .flags
            .unwrap_or_default()
            .contains(serenity::MessageFlags::EPHEMERAL)
        {
            msg.edit(ctx.http(), EditMessage::default().components(vec![]))
                .await?;
        }
    }

    Ok(())
}

/// Check DC travel status for a world
#[poise::command(slash_command)]
async fn world(
    ctx: Context<'_>,
    #[description = "World to check for"]
    #[autocomplete = "autocomplete_world"]
    world: u16,
) -> Result<(), Error> {
    let world = worlds::get_data()
        .get_world_by_id(world)
        .cloned()
        .ok_or(Error::UnknownWorld)?;

    let client = ctx.data();
    let db = client.db();
    let config = client.config();
    let is_prohibited = db::travel::get_travel_states_by_world_id(db, vec![world.id])
        .await?
        .get(&world.id)
        .copied()
        .unwrap_or_default();

    let embed = create_travel_embed(&world.to_string(), vec![(&world, is_prohibited)], config);

    let components = if is_prohibited {
        vec![serenity::CreateActionRow::Buttons(vec![
            serenity::CreateButton::new("set_reminder")
                .label("Remind Me")
                .emoji(ReactionType::Unicode("⏰".to_owned()))
                .style(poise::serenity_prelude::ButtonStyle::Primary),
        ])]
    } else {
        vec![]
    };

    let reply = ctx
        .send(
            CreateReply::default()
                .reply(true)
                .embed(embed)
                .components(components),
        )
        .await?;

    if is_prohibited {
        if let Some(interaction) = reply
            .message()
            .await?
            .await_component_interaction(ctx)
            .author_id(ctx.author().id)
            .timeout(std::time::Duration::from_secs(120))
            .await
        {
            if interaction.data.custom_id == "set_reminder" {
                subscribe_world(ctx, world, true).await?;
            }
            interaction
                .create_response(ctx, serenity::CreateInteractionResponse::Acknowledge)
                .await?;
        }
        let mut msg = reply.into_message().await?;
        if !msg
            .flags
            .unwrap_or_default()
            .contains(serenity::MessageFlags::EPHEMERAL)
        {
            msg.edit(ctx.http(), EditMessage::default().components(vec![]))
                .await?;
        }
    }

    Ok(())
}
