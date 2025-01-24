use super::DiscordClient;

mod admin;
mod queue_times;
mod stats;
mod subscribe;
mod travel;
mod unsubscribe;
mod utils;

pub use utils::create_travel_embed;

pub type Data = DiscordClient;
pub type Context<'a> = poise::Context<'a, Data, Error>;
pub type Command = poise::Command<Data, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Serenity error")]
    Serenity(#[from] serenity::Error),
    #[error("Database error")]
    Database(#[from] sqlx::Error),
    #[error("Native error")]
    Native(#[from] crate::natives::Error),
    #[error("Subscription error")]
    Subscription(#[from] crate::subscriptions::Error),
    #[error("Unknown world")]
    UnknownWorld,
    #[error("Unknown datacenter")]
    UnknownDatacenter,
    #[error("Admin error")]
    Admin,
}

pub fn command_list() -> Vec<Command> {
    vec![
        travel::travel(),
        queue_times::queue_times(),
        subscribe::subscribe(),
        unsubscribe::unsubscribe(),
        stats::stats(),
        admin::admin(),
    ]
}
