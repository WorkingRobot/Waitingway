use actix_files::Files;
use actix_web::dev::HttpServiceFactory;

pub fn service() -> impl HttpServiceFactory {
    Files::new("/", "static").index_file("index.html")
}
