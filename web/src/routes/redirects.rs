use actix_web::{dev::HttpServiceFactory, get, http::header, HttpResponse, Result};

pub fn service() -> impl HttpServiceFactory {
    (discord, funding)
}

#[get("/discord/")]
async fn discord() -> Result<HttpResponse> {
    Ok(HttpResponse::MovedPermanently()
        .insert_header((header::LOCATION, "https://discord.gg/3PGKKWYTGc"))
        .finish())
}

#[get("/funding/")]
async fn funding() -> Result<HttpResponse> {
    Ok(HttpResponse::MovedPermanently()
        .insert_header((header::LOCATION, "https://ko-fi.com/camora"))
        .finish())
}
