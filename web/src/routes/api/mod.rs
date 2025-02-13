mod base;
mod connections;
mod notifications;
mod oauth;
mod queue;
mod summary;
mod travel;
mod world_status;

use actix_web::{dev::HttpServiceFactory, web};

fn v1() -> impl HttpServiceFactory {
    web::scope("/v1")
        .service(base::service())
        .service(travel::service())
        .service(world_status::service())
        .service(summary::service())
        .service(oauth::service())
        .service(connections::service())
        .service(queue::login::service_v1())
}

fn v2() -> impl HttpServiceFactory {
    web::scope("/v2").service(queue::service())
}

pub fn service() -> impl HttpServiceFactory {
    web::scope("/api").service(v1()).service(v2())
}
