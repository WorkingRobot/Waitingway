use crate::{auth::BasicAuthentication, db, models::Recap};
use actix_web::{
    dev::HttpServiceFactory, error::ErrorInternalServerError, get, route, web, HttpResponse, Result,
};
use sqlx::PgPool;
use uuid::Uuid;

pub fn service() -> impl HttpServiceFactory {
    (create_recap, get_queue)
}

#[route("/recap/", method = "POST", wrap = "BasicAuthentication")]
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

#[get("/queue/")]
async fn get_queue() -> Result<HttpResponse> {
    Ok(HttpResponse::Ok().finish())
}
