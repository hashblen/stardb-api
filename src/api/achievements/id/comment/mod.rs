use actix_session::Session;
use actix_web::{delete, put, web, HttpResponse, Responder};
use serde::Deserialize;
use sqlx::PgPool;
use utoipa::{OpenApi, ToSchema};

use crate::{database, Result};

#[derive(OpenApi)]
#[openapi(
    paths(put_achievement_comment, delete_achievement_comment),
    components(schemas(CommentUpdate))
)]
struct ApiDoc;

#[derive(Deserialize, ToSchema)]
struct CommentUpdate {
    comment: String,
}

pub fn openapi() -> utoipa::openapi::OpenApi {
    ApiDoc::openapi()
}

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(put_achievement_comment)
        .service(delete_achievement_comment);
}

#[utoipa::path(
    tag = "achievements/{id}",
    put,
    path = "/api/achievements/{id}/comment",
    request_body = CommentUpdate,
    responses(
        (status = 200, description = "Updated comment"),
        (status = 403, description = "Not an admin"),
    ),
    security(("admin" = []))
)]
#[put("/api/achievements/{id}/comment")]
async fn put_achievement_comment(
    session: Session,
    id: web::Path<i64>,
    comment_update: web::Json<CommentUpdate>,
    pool: web::Data<PgPool>,
) -> Result<impl Responder> {
    let Ok(Some(admin)) = session.get::<bool>("admin") else {
        return Ok(HttpResponse::BadRequest().finish());
    };

    if !admin {
        return Ok(HttpResponse::Forbidden().finish());
    }

    database::update_achievement_comment(*id, &comment_update.comment, &pool).await?;

    Ok(HttpResponse::Ok().finish())
}

#[utoipa::path(
    tag = "achievements/{id}",
    delete,
    path = "/api/achievements/{id}/comment",
    responses(
        (status = 200, description = "Deleted comment"),
        (status = 403, description = "Not an admin"),
    ),
    security(("admin" = []))
)]
#[delete("/api/achievements/{id}/comment")]
async fn delete_achievement_comment(
    session: Session,
    id: web::Path<i64>,
    pool: web::Data<PgPool>,
) -> Result<impl Responder> {
    let Ok(Some(admin)) = session.get::<bool>("admin") else {
        return Ok(HttpResponse::BadRequest().finish());
    };

    if !admin {
        return Ok(HttpResponse::Forbidden().finish());
    }

    database::delete_achievement_comment(*id, &pool).await?;

    Ok(HttpResponse::Ok().finish())
}
