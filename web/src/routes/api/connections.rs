use crate::{auth::BasicAuthentication, db, discord::DiscordClient};
use actix_web::{
    dev::HttpServiceFactory,
    error::{ErrorInternalServerError, ErrorNotFound},
    route, web, HttpResponse, Result,
};
use serenity::all::UserId;
use sqlx::PgPool;
use uuid::Uuid;

pub fn service() -> impl HttpServiceFactory {
    web::scope("/connections")
        .service(get_connections)
        .service(get_connections)
        .service(delete_connection)
}

#[route("/", method = "GET", wrap = "BasicAuthentication")]
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

#[route("/{id}/", method = "DELETE", wrap = "BasicAuthentication")]
async fn delete_connection(
    pool: web::Data<PgPool>,
    discord: web::Data<DiscordClient>,
    username: web::ReqData<Uuid>,
    id: web::Path<u64>,
) -> Result<HttpResponse> {
    let id = id.into_inner();
    let resp = db::delete_connection(&pool, *username, id)
        .await
        .map_err(|e| ErrorInternalServerError(e))?;

    if resp.rows_affected() == 0 {
        return Err(ErrorNotFound("Connection not found"));
    }

    discord
        .offboard_user(UserId::new(id))
        .await
        .map_err(|e| ErrorInternalServerError(e))?;

    Ok(HttpResponse::NoContent().finish())
}
