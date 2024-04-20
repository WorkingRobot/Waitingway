mod auth;
mod config;
mod db;
mod discord;
mod models;
mod oauth;
mod routes;

use std::io;

use ::config::{Config, Environment};
use actix_cors::Cors;
use actix_web::{
    middleware::{Logger, NormalizePath, TrailingSlash},
    web::Data,
    App, HttpServer,
};
use thiserror::Error;

use crate::discord::DiscordClient;

#[derive(Error, Debug)]
pub enum ServerError {
    #[error("Serenity error")]
    SerenityError(#[from] serenity::Error),
    #[error("Join error")]
    JoinError(#[from] tokio::task::JoinError),
    #[error("Actix error")]
    ActixError(#[from] io::Error),
    #[error("Reqwest error")]
    ReqwestError(#[from] reqwest::Error),
    #[error("Dotenvy error")]
    DotenvyError(#[from] dotenvy::Error),
}

#[tokio::main]
async fn main() -> Result<(), ServerError> {
    #[cfg(debug_assertions)]
    {
        dotenvy::dotenv()?;
        dotenvy::from_filename(".secrets.env")?;
    }

    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    let config: config::Config = Config::builder()
        .add_source(Environment::default())
        .build()
        .and_then(|v| v.try_deserialize())
        .unwrap();

    sqlx::any::install_default_drivers();
    let db_pool = sqlx::postgres::PgPoolOptions::new()
        .connect(&config.database_url)
        .await
        .unwrap();

    sqlx::migrate!().run(&db_pool).await.unwrap();

    let discord_bot = DiscordClient::new(config.discord.clone()).await;

    let server_pool = db_pool.clone();
    let server_config = config.clone();
    let server_discord = discord_bot.clone();
    let server = HttpServer::new(move || {
        App::new()
            .wrap(Cors::default())
            .wrap(NormalizePath::new(TrailingSlash::Always))
            .wrap(Logger::default())
            .app_data(Data::new(server_pool.clone()))
            .app_data(Data::new(server_config.clone()))
            .app_data(Data::new(server_discord.clone()))
            .app_data(Data::new(
                reqwest::Client::builder()
                    .user_agent("Waitingway")
                    .build()
                    .expect("Error creating reqwest client"),
            ))
            .service(routes::redirects::service())
            .service(routes::api::service())
    })
    .bind(config.server_addr.clone())?
    .run();

    println!("Http server running at http://{}", config.server_addr);

    let discord_task_bot = discord_bot.clone();
    let discord_task = tokio::task::spawn(async move { discord_task_bot.start().await });
    let server_task = tokio::task::spawn(server);

    let server_ret = server_task.await;
    discord_bot.stop().await;
    let discord_ret = discord_task.await;

    db_pool.close().await;

    server_ret??;
    discord_ret??;

    println!("Goodbye!");

    Ok(())
}
