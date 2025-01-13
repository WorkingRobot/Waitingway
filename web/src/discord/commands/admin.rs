use super::Context;
use super::Error;
use ::serenity::all::{
    CreateActionRow, CreateAllowedMentions, CreateInputText, CreateInteractionResponse,
    CreateMessage, CreateQuickModal, CreateSelectMenu, ReactionType, Role, RoleId,
};
use itertools::Itertools;
use poise::serenity_prelude as serenity;

#[derive(Debug, poise::ChoiceParameter)]
pub enum Subcommand {
    #[name = "Create message"]
    CreateMessage,
    #[name = "Create role message"]
    CreateRoleMessage,
}

#[poise::command(
    slash_command,
    install_context = "Guild",
    interaction_context = "Guild",
    required_permissions = "ADMINISTRATOR",
    default_member_permissions = "ADMINISTRATOR",
    identifying_name = "internal_admin",
    owners_only,
    guild_only,
    ephemeral
)]
pub async fn admin(
    ctx: Context<'_>,
    subcommand: Subcommand,
    #[channel_types("Text")] channel: serenity::GuildChannel,
) -> Result<(), Error> {
    let poise::Context::Application(ctx) = ctx else {
        return Err(Error::Admin);
    };
    match subcommand {
        Subcommand::CreateMessage => {
            let modal = CreateQuickModal::new("Message Content")
                .timeout(std::time::Duration::from_secs(30))
                .field(
                    CreateInputText::new(serenity::InputTextStyle::Paragraph, "Content", "")
                        .max_length(2000),
                );
            let response = ctx
                .interaction
                .quick_modal(ctx.serenity_context, modal)
                .await?
                .ok_or(Error::Admin)?;
            let (content,) = response
                .inputs
                .into_iter()
                .collect_tuple()
                .ok_or(Error::Admin)?;
            channel
                .send_message(
                    ctx.serenity_context,
                    CreateMessage::new()
                        .content(content)
                        .allowed_mentions(CreateAllowedMentions::new()),
                )
                .await?;
            response
                .interaction
                .create_response(ctx.http(), CreateInteractionResponse::Acknowledge)
                .await?;
        }
        Subcommand::CreateRoleMessage => {
            let modal = CreateQuickModal::new("Message Content")
                .timeout(std::time::Duration::from_secs(3600))
                .field(
                    CreateInputText::new(serenity::InputTextStyle::Paragraph, "Content", "")
                        .max_length(2000),
                )
                .paragraph_field("Role Id List")
                .paragraph_field("Role Emoji List")
                .paragraph_field("Role Description List");
            let response = ctx
                .interaction
                .quick_modal(ctx.serenity_context, modal)
                .await?
                .ok_or(Error::Admin)?;
            let (content, role_ids, role_emojis, role_descs) = response
                .inputs
                .into_iter()
                .collect_tuple()
                .ok_or(Error::Admin)?;
            let guild_roles = ctx
                .guild_id()
                .ok_or(Error::Admin)?
                .roles(ctx.http())
                .await?;
            let roles: Vec<&Role> = role_ids
                .split_whitespace()
                .map(|id| {
                    id.parse::<u64>()
                        .map_err(|_| Error::Admin)
                        .map(RoleId::new)
                        .and_then(|r| guild_roles.get(&r).ok_or(Error::Admin))
                })
                .collect::<Result<_, _>>()?;
            let role_emojis: Vec<ReactionType> = role_emojis
                .split_whitespace()
                .map(ReactionType::try_from)
                .collect::<Result<_, _>>()
                .map_err(|_| Error::Admin)?;
            let role_descs = role_descs.split('\n').map(str::to_string).collect_vec();
            let menu_options = roles
                .iter()
                .zip(role_emojis.iter())
                .zip(role_descs.iter())
                .map(|((&role, emoji), description)| {
                    serenity::CreateSelectMenuOption::new(
                        role.name.clone(),
                        role.id.get().to_string(),
                    )
                    .emoji(emoji.clone())
                    .description(description)
                })
                .collect_vec();
            let menu_count = menu_options.len() as u8;
            let select_menu = CreateActionRow::SelectMenu(
                CreateSelectMenu::new(
                    "role_selector",
                    serenity::CreateSelectMenuKind::String {
                        options: menu_options,
                    },
                )
                .min_values(0)
                .max_values(menu_count)
                .placeholder("Select Roles"),
            );
            channel
                .send_message(
                    ctx.serenity_context,
                    CreateMessage::new()
                        .content(content)
                        .components(vec![select_menu])
                        .allowed_mentions(CreateAllowedMentions::new()),
                )
                .await?;
            response
                .interaction
                .create_response(ctx.http(), CreateInteractionResponse::Acknowledge)
                .await?;
        }
    }
    Ok(())
}
