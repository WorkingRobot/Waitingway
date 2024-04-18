use crate::{auth::BasicAuthentication, db, models::Recap};
use actix_web::{
    dev::HttpServiceFactory,
    error::{ErrorInternalServerError, ErrorNotFound},
    get, route, web, HttpResponse, Result,
};
use sqlx::PgPool;
use uuid::Uuid;

pub fn service() -> impl HttpServiceFactory {
    web::scope("/api/v1")
        .service(get_connections)
        .service(delete_connection)
        .service(create_recap)
        .service(get_queue)
}

#[route("/connections", method = "GET", wrap = "BasicAuthentication")]
async fn get_connections(
    pool: web::Data<PgPool>,
    username: web::ReqData<Uuid>,
) -> Result<HttpResponse> {
    let connections = db::get_connections_by_user_id(pool.get_ref(), *username).await;
    match connections {
        Ok(connections) => Ok(HttpResponse::Ok().json(connections)),
        Err(e) => Err(ErrorInternalServerError(e)),
    }
}

#[route("/connections/{id}", method = "DELETE", wrap = "BasicAuthentication")]
async fn delete_connection(
    pool: web::Data<PgPool>,
    username: web::ReqData<Uuid>,
    id: web::Path<i64>,
) -> Result<HttpResponse> {
    let resp = db::delete_connection(&pool, *username, id.into_inner()).await;
    match resp {
        Ok(query) => {
            if query.rows_affected() == 0 {
                Err(ErrorNotFound("Connection not found"))
            } else {
                Ok(HttpResponse::NoContent().finish())
            }
        }
        Err(e) => Err(ErrorInternalServerError(e)),
    }
}

#[route("/recap", method = "POST", wrap = "BasicAuthentication")]
async fn create_recap(
    pool: web::Data<PgPool>,
    username: web::ReqData<Uuid>,
    recap: web::Json<Recap>,
) -> Result<HttpResponse> {
    let mut recap = recap.into_inner();
    recap.user_id = *username;
    recap.id = Uuid::now_v7();

    let resp = db::create_recap(&pool, recap).await;
    match resp {
        Ok(_) => Ok(HttpResponse::Created().finish()),
        Err(e) => Err(ErrorInternalServerError(e)),
    }
}

#[get("/queue")]
async fn get_queue() -> Result<HttpResponse> {
    Ok(HttpResponse::Ok().finish())
}
