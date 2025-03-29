mod base;
mod connections;
mod notifications;
mod oauth;
mod queue;
mod summary;
mod travel;
mod world_status;

use actix_web::{
    body::{EitherBody, MessageBody},
    dev::{HttpServiceFactory, ServiceResponse},
    middleware::{ErrorHandlerResponse, ErrorHandlers},
    web::{self, Bytes},
};

fn v1() -> impl HttpServiceFactory {
    web::scope("/v1")
        .service(base::service())
        .service(travel::service())
        .service(world_status::service())
        .service(summary::service())
        .service(oauth::service())
        .service(connections::service())
}

fn v2() -> impl HttpServiceFactory {
    web::scope("/v2").service(queue::service())
}

pub fn service() -> impl HttpServiceFactory {
    web::scope("/api").service(v1()).service(v2()).wrap(
        ErrorHandlers::new()
            .default_handler_client(|r| log_error(true, r))
            .default_handler_server(|r| log_error(false, r)),
    )
}

fn log_error<B: MessageBody + 'static>(
    is_client: bool,
    res: ServiceResponse<B>,
) -> actix_web::Result<ErrorHandlerResponse<B>> {
    Ok(ErrorHandlerResponse::Future(Box::pin(log_error2(
        is_client, res,
    ))))
}

async fn log_error2<B: MessageBody + 'static>(
    is_client: bool,
    res: ServiceResponse<B>,
) -> actix_web::Result<ServiceResponse<EitherBody<B>>> {
    let (req, res) = res.into_parts();
    let (res, body) = res.into_parts();

    let body = {
        let data = actix_web::body::to_bytes_limited(body, 1 << 12).await;
        let line = match &data {
            Ok(Ok(data)) => String::from_utf8_lossy(data).into_owned(),
            Ok(Err(_)) => "Error reading body".to_string(),
            Err(_) => "Body too large".to_string(),
        };
        if is_client {
            log::error!("Client Error: {}", line);
        } else {
            log::error!("Server Error: {}", line);
        }

        match data {
            Ok(Ok(bytes)) => bytes,
            Ok(Err(_)) => Bytes::from_static(b"Body conversion failure"),
            Err(_) => Bytes::from_static(b"Body too large"),
        }
    };

    let res = ServiceResponse::new(req, res.map_body(|_head, _body| body))
        .map_into_boxed_body()
        .map_into_right_body();

    Ok(res)
}
