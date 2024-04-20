mod base;
mod connections;
mod notifications;
mod oauth;

use actix_web::{dev::HttpServiceFactory, web};

pub fn service() -> impl HttpServiceFactory {
    web::scope("/api/v1")
        .service(base::service())
        .service(oauth::service())
        .service(connections::service())
        .service(notifications::service())
}
