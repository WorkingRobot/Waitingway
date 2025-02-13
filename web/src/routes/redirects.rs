use crate::config::Config;
use actix_web::{
    dev::HttpServiceFactory,
    get,
    http::header,
    web::{self, Redirect},
    HttpResponse, Responder, Result,
};

pub fn service() -> impl HttpServiceFactory {
    (discord, funding, github)
}

#[get("/discord/")]
async fn discord(config: web::Data<Config>) -> Result<impl Responder> {
    Ok(Redirect::to(format!(
        "https://discord.gg/{}",
        config.discord.guild_invite_code
    ))
    .permanent())
}

#[get("/funding/")]
async fn funding() -> Result<impl Responder> {
    Ok(Redirect::to("https://ko-fi.com/camora").permanent())
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
