mod auth;
mod config;
mod db;
mod models;
mod oauth;
mod routes;

use ::config::{Config, Environment};
use actix_cors::Cors;
use actix_web::{web::Data, App, HttpServer};
use awc::http::header;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenvy::dotenv().ok();

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

    let server_pool = db_pool.clone();
    let server_config = config.clone();
    let server = HttpServer::new(move || {
        App::new()
            .wrap(Cors::default())
            .app_data(Data::new(server_pool.clone()))
            .app_data(Data::new(server_config.clone()))
            .app_data(Data::new(
                awc::Client::builder()
                    .add_default_header((header::USER_AGENT, "Waitingway"))
                    .finish(),
            ))
            .service(routes::redirects::service())
            .service(routes::api::service())
            .service(routes::oauth::service())
    })
    .bind(config.server_addr.clone())?
    .run();

    println!("Server running at http://{}", config.server_addr);

    let ret = server.await;
    db_pool.close().await;

    ret
}
