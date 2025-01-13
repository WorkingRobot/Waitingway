use crate::config::Config;
use actix_web::{dev::HttpServiceFactory, get, http::header, web, HttpResponse, Result};

pub fn service() -> impl HttpServiceFactory {
    (discord, funding, github)
}

#[get("/discord/")]
async fn discord(config: web::Data<Config>) -> Result<HttpResponse> {
    Ok(HttpResponse::MovedPermanently()
        .insert_header((
            header::LOCATION,
            format!("https://discord.gg/{}", config.discord.guild_invite_code),
        ))
        .finish())
}

#[get("/funding/")]
async fn funding() -> Result<HttpResponse> {
    Ok(HttpResponse::MovedPermanently()
        .insert_header((header::LOCATION, "https://ko-fi.com/camora"))
        .finish())
}

#[get("/github/")]
async fn github() -> Result<HttpResponse> {
    Ok(HttpResponse::MovedPermanently()
        .insert_header((
            header::LOCATION,
            "https://github.com/WorkingRobot/Waitingway",
        ))
        .finish())
}
