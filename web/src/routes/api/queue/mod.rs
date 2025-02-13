use actix_web::{dev::HttpServiceFactory, web};

mod duty;
pub mod login;

pub fn service() -> impl HttpServiceFactory {
    web::scope("/queue")
        .service(duty::service())
        .service(login::service())
}
