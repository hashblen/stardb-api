mod damage;
mod heal;
mod shield;

use actix_web::web;
mod achievements;
use serde::{Deserialize, Serialize};
use strum::{Display, EnumString};
use utoipa::{IntoParams, OpenApi, ToSchema};

#[derive(OpenApi)]
#[openapi(components(schemas(Region)))]
struct ApiDoc;

#[derive(Display, EnumString, Serialize, Deserialize, ToSchema, Clone, Copy)]
#[strum(serialize_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum Region {
    NA,
    EU,
    Asia,
    CN,
}

#[derive(Deserialize, IntoParams)]
pub struct ScoresParams {
    pub region: Option<Region>,
    pub query: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

pub fn openapi() -> utoipa::openapi::OpenApi {
    let mut openapi = ApiDoc::openapi();
    openapi.merge(achievements::openapi());
    openapi.merge(damage::openapi());
    openapi.merge(heal::openapi());
    openapi.merge(shield::openapi());
    openapi
}

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.configure(achievements::configure)
        .configure(damage::configure)
        .configure(heal::configure)
        .configure(shield::configure);
}
