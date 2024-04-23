mod auth;
mod config;
mod db;
mod discord;
mod models;
mod oauth;
mod routes;

use crate::discord::DiscordClient;
use ::config::{Config, Environment, File, FileFormat};
use actix_cors::Cors;
use actix_web::{
    middleware::{Logger, NormalizePath, TrailingSlash},
    web::Data,
    App, HttpServer,
};
use actix_web_prom::PrometheusMetricsBuilder;
use prometheus::Registry;
use std::io;
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
}

#[tokio::main]
async fn main() -> Result<(), ServerError> {
    #[cfg(debug_assertions)]
    {
        _ = dotenvy::from_filename(".env");
        _ = dotenvy::from_filename(".secrets.env")?;
    }

    let config: config::Config = Config::builder()
        .add_source(File::new("config", FileFormat::Yaml))
        .add_source(Environment::default())
        .build()
        .and_then(|v| v.try_deserialize())
        .unwrap();

    env_logger::init_from_env(
        env_logger::Env::new()
            .default_filter_or(config.clone().log_filter.unwrap_or("info".to_string())),
    );

    sqlx::any::install_default_drivers();
    let db_pool = sqlx::postgres::PgPoolOptions::new()
        .connect(&config.database_url)
        .await
        .unwrap();

    sqlx::migrate!().run(&db_pool).await.unwrap();

    let discord_bot = DiscordClient::new(config.discord.clone()).await;

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
        App::new()
            .wrap(private_prometheus.clone())
            .wrap(Logger::default())
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
