#![allow(clippy::too_many_lines, clippy::redundant_closure_for_method_calls)]

mod cache;
mod config;
mod crons;
mod discord;
mod middleware;
mod models;
mod natives;
mod oauth;
mod routes;
mod stopwatch;
mod storage;
mod subscriptions;

use crate::discord::DiscordClient;
use ::config::{Config, Environment, File, FileFormat};
use actix_cors::Cors;
use actix_web::{
    App, HttpServer,
    middleware::{Logger, NormalizePath, TrailingSlash},
    web::Data,
};
use actix_web_prom::PrometheusMetricsBuilder;
use natives::version;
use prometheus::Registry;
use std::io;
use storage::redis::client::RedisClient;
use thiserror::Error;

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
    #[error("Prometheus error")]
    PrometheusError(#[from] prometheus::Error),
    #[error("Game data error")]
    GameDataError(#[from] storage::game::GameDataError),
}

#[tokio::main]
async fn main() -> Result<(), ServerError> {
    #[cfg(debug_assertions)]
    {
        _ = dotenvy::from_filename(".env");
        _ = dotenvy::from_filename(".secrets.env")?;
        unsafe { std::env::set_var("RUST_BACKTRACE", "1") };
    }

    let config: config::Config = Config::builder()
        .add_source(File::new("config", FileFormat::Yaml))
        .add_source(Environment::default())
        .build()
        .and_then(Config::try_deserialize)
        .unwrap();

    env_logger::init_from_env(
        env_logger::Env::new()
            .default_filter_or(config.log_filter.clone().unwrap_or("info".to_string())),
    );

    let db_pool = sqlx::postgres::PgPoolOptions::new()
        .connect(&config.database_url)
        .await
        .unwrap();

    sqlx::migrate!().run(&db_pool).await.unwrap();

    let redis = RedisClient::new(config.redis.clone())
        .await
        .expect("Error creating redis client");

    let web_client = reqwest::Client::builder()
        .user_agent(version().to_string())
        .build()
        .expect("Error creating reqwest client");

    storage::game::initialize(&db_pool, &web_client).await?;

    let discord_bot =
        DiscordClient::new(config.discord.clone(), db_pool.clone(), redis.clone()).await;

    let update_activity_token =
        crons::create_cron_job(crons::UpdateActivity::new(discord_bot.clone()));

    let refresh_queue_estimates_token =
        crons::create_cron_job(crons::RefreshMaterializedViews::new(db_pool.clone()));

    let refresh_travel_states_token = crons::create_cron_job(
        crons::RefreshTravelStates::new(
            config.stasis.clone(),
            db_pool.clone(),
            discord_bot.subscriptions().clone(),
        )
        .expect("Error creating travel states cron job"),
    );

    let refresh_world_states_token = crons::create_cron_job(crons::RefreshWorldStatuses::new(
        web_client.clone(),
        db_pool.clone(),
    ));

    let prometheus_registry = Registry::new();

    let server_prometheus = PrometheusMetricsBuilder::new("public")
        .registry(prometheus_registry.clone())
        .build()
        .map_err(|e| {
            *e.downcast::<prometheus::Error>()
                .expect("Unknown error from prometheus builder")
        })?;
    let server_pool = db_pool.clone();
    let server_config = config.clone();
    let server_discord = discord_bot.clone();
    let server = HttpServer::new(move || {
        App::new()
            .wrap(Cors::default())
            .wrap(NormalizePath::new(TrailingSlash::Always))
            .wrap(server_prometheus.clone())
            .wrap(
                server_config
                    .log_access_format
                    .as_deref()
                    .map_or_else(Logger::default, Logger::new),
            )
            .app_data(Data::new(server_pool.clone()))
            .app_data(Data::new(server_config.clone()))
            .app_data(Data::new(server_discord.clone()))
            .app_data(Data::new(web_client.clone()))
            .app_data(Data::new(redis.clone()))
            .service(routes::api::service())
            .service(routes::redirects::service())
            .service(routes::assets::service())
    })
    .bind(config.server_addr.clone())?
    .run();

    log::info!("Http server running at http://{}", config.server_addr);

    let private_prometheus = PrometheusMetricsBuilder::new("private")
        .registry(prometheus_registry)
        .endpoint("/metrics")
        .build()
        .map_err(|e| {
            *e.downcast::<prometheus::Error>()
                .expect("Unknown error from prometheus builder")
        })?;
    let prometheus_server = HttpServer::new(move || {
        App::new().wrap(private_prometheus.clone()).wrap(
            config
                .log_access_format
                .as_deref()
                .map_or_else(Logger::default, Logger::new),
        )
    })
    .workers(1)
    .bind(config.metrics_server_addr.clone())?
    .run();

    log::info!(
        "Metrics http server running at http://{}",
        config.metrics_server_addr
    );

    let discord_task_bot = discord_bot.clone();
    let discord_task = tokio::task::spawn(async move { discord_task_bot.start().await });
    let server_task = tokio::task::spawn(server);
    let prometheus_server_task = tokio::task::spawn(prometheus_server);

    let server_ret = server_task.await;

    refresh_queue_estimates_token.cancel();
    refresh_travel_states_token.cancel();
    refresh_world_states_token.cancel();
    update_activity_token.cancel();
    discord_bot.stop().await;
    let prometheus_server_ret = prometheus_server_task.await;
    let discord_ret = discord_task.await;

    db_pool.close().await;

    server_ret??;
    prometheus_server_ret??;
    discord_ret??;

    log::info!("Goodbye!");

    Ok(())
}
