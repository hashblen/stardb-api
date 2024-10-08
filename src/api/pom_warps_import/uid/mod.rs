use actix_session::Session;
use actix_web::{post, web, HttpResponse, Responder};
use chrono::NaiveDateTime;
use sqlx::PgPool;
use utoipa::OpenApi;

use crate::{api::ApiResult, database, mihomo, GachaType, Language};

#[derive(OpenApi)]
#[openapi(
    tags((name = "pom-warps-import/{uid}")),
    paths(post_pom_warps_import),
    components(schemas(PomWarpsImportParams)),
)]
struct ApiDoc;

pub fn openapi() -> utoipa::openapi::OpenApi {
    ApiDoc::openapi()
}

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(post_pom_warps_import);
}

#[derive(serde::Deserialize, utoipa::ToSchema)]
struct PomWarpsImportParams {
    data: String,
}

#[derive(serde::Deserialize)]
struct Pom {
    default: Default,
}

#[derive(serde::Deserialize)]
struct Default {
    #[serde(rename = "beginner")]
    departure: Vec<Warp>,
    #[serde(rename = "standard")]
    standard: Vec<Warp>,
    #[serde(rename = "character")]
    special: Vec<Warp>,
    #[serde(rename = "lightcone")]
    lc: Vec<Warp>,
}

#[derive(serde::Deserialize)]
struct Warp {
    id: String,
    #[serde(rename = "itemId")]
    item_id: String,
    time: String,
}

#[utoipa::path(
    tag = "pom-warps-import/{uid}",
    post,
    path = "/api/pom-warps-import/{uid}",
    request_body = PomWarpsImportParams,
    responses(
        (status = 200, description = "Warps imported"),
        (status = 403, description = "Not an admin"),
    ),
    security(("admin" = []))
)]
#[post("/api/pom-warps-import/{uid}")]
async fn post_pom_warps_import(
    session: Session,
    uid: web::Path<i32>,
    params: web::Json<PomWarpsImportParams>,
    pool: web::Data<PgPool>,
) -> ApiResult<impl Responder> {
    let Ok(Some(username)) = session.get::<String>("username") else {
        return Ok(HttpResponse::BadRequest().finish());
    };

    if !database::admins::exists(&username, &pool).await? {
        return Ok(HttpResponse::Forbidden().finish());
    }

    let uid = *uid;

    // Wacky way to update the database in case the uid isn't in there
    if !database::mihomo::exists(uid, &pool).await?
        && mihomo::get(uid, Language::En, &pool).await.is_err()
    {
        let region = match uid.to_string().chars().next() {
            Some('6') => "na",
            Some('7') => "eu",
            Some('8') | Some('9') => "asia",
            _ => "cn",
        }
        .to_string();

        let db_mihomo = database::mihomo::DbMihomo {
            uid,
            region,
            ..std::default::Default::default()
        };

        database::mihomo::set(&db_mihomo, &pool).await?;
    }

    let timestamp_offset = chrono::Duration::hours(match uid.to_string().chars().next() {
        Some('6') => -5,
        Some('7') => 1,
        _ => 8,
    });

    let pom: Pom = serde_json::from_str(&params.data)?;

    let mut set_all_departure = database::warps::SetAll::default();
    let mut set_all_standard = database::warps::SetAll::default();
    let mut set_all_special = database::warps::SetAll::default();
    let mut set_all_lc = database::warps::SetAll::default();

    for (warps, gacha_type) in [
        (&pom.default.departure, GachaType::Departure),
        (&pom.default.standard, GachaType::Standard),
        (&pom.default.special, GachaType::Special),
        (&pom.default.lc, GachaType::Lc),
    ] {
        for warp in warps {
            let id = warp.id.parse::<i64>().unwrap();
            let item_id = warp.item_id.parse().unwrap();
            let (character, light_cone) = if item_id < 2000 {
                (Some(item_id), None)
            } else {
                (None, Some(item_id))
            };

            let timestamp = NaiveDateTime::parse_from_str(&warp.time, "%Y-%m-%d %H:%M:%S")?
                .and_utc()
                - timestamp_offset;

            let set_all = match gacha_type {
                GachaType::Departure => &mut set_all_departure,
                GachaType::Standard => &mut set_all_standard,
                GachaType::Special => &mut set_all_special,
                GachaType::Lc => &mut set_all_lc,
            };

            set_all.id.push(id);
            set_all.uid.push(uid);
            set_all.character.push(character);
            set_all.light_cone.push(light_cone);
            set_all.timestamp.push(timestamp);
            set_all.official.push(false);
        }
    }

    database::warps::departure::set_all(&set_all_departure, &pool).await?;
    database::warps::standard::set_all(&set_all_standard, &pool).await?;
    database::warps::special::set_all(&set_all_special, &pool).await?;
    database::warps::lc::set_all(&set_all_lc, &pool).await?;

    calculate_stats_standard(uid, &pool).await?;
    calculate_stats_special(uid, &pool).await?;
    calculate_stats_lc(uid, &pool).await?;

    Ok(HttpResponse::Ok().finish())
}

async fn calculate_stats_standard(uid: i32, pool: &PgPool) -> anyhow::Result<()> {
    let warps = database::warps::standard::get_infos_by_uid(uid, pool).await?;

    let mut pull_4 = 0;
    let mut sum_4 = 0;
    let mut count_4 = 0;

    let mut pull_5 = 0;
    let mut sum_5 = 0;
    let mut count_5 = 0;

    for warp in &warps {
        pull_4 += 1;
        pull_5 += 1;

        match warp.rarity.unwrap() {
            4 => {
                count_4 += 1;
                sum_4 += pull_4;
                pull_4 = 0;
            }
            5 => {
                count_5 += 1;
                sum_5 += pull_5;
                pull_5 = 0;
            }
            _ => {}
        }
    }

    let luck_4 = sum_4 as f64 / count_4 as f64;
    let luck_5 = sum_5 as f64 / count_5 as f64;

    let stat = database::warps_stats::standard::DbWarpsStatStandard {
        uid,
        luck_4,
        luck_5,
    };
    database::warps_stats::standard::set(&stat, pool).await?;

    Ok(())
}

async fn calculate_stats_special(uid: i32, pool: &PgPool) -> anyhow::Result<()> {
    let warps = database::warps::special::get_infos_by_uid(uid, pool).await?;

    let mut pull_4 = 0;
    let mut sum_4 = 0;
    let mut count_4 = 0;

    let mut pull_5 = 0;
    let mut sum_5 = 0;
    let mut count_5 = 0;

    let mut guarantee = false;

    let mut sum_win = 0;
    let mut count_win = 0;

    let mut win_streak = 0;
    let mut max_win_streak = 0;

    let mut loss_streak = 0;
    let mut max_loss_streak = 0;

    for warp in &warps {
        pull_4 += 1;
        pull_5 += 1;

        match warp.rarity.unwrap() {
            4 => {
                count_4 += 1;
                sum_4 += pull_4;
                pull_4 = 0;
            }
            5 => {
                count_5 += 1;
                sum_5 += pull_5;
                pull_5 = 0;

                if guarantee {
                    guarantee = false;
                } else {
                    count_win += 1;

                    if [1209, 1004, 1101, 1211, 1104, 1107, 1003].contains(&warp.character.unwrap())
                    {
                        win_streak = 0;

                        loss_streak += 1;
                        max_loss_streak = max_loss_streak.max(loss_streak);

                        guarantee = true;
                    } else {
                        sum_win += 1;

                        loss_streak = 0;

                        win_streak += 1;
                        max_win_streak = max_win_streak.max(win_streak);
                    }
                }
            }
            _ => {}
        }
    }

    let win_streak = max_win_streak;
    let loss_streak = max_loss_streak;

    let luck_4 = sum_4 as f64 / count_4 as f64;
    let luck_5 = sum_5 as f64 / count_5 as f64;
    let win_rate = sum_win as f64 / count_win as f64;

    let stat = database::warps_stats::special::DbWarpsStatSpecial {
        uid,
        luck_4,
        luck_5,
        win_rate,
        win_streak,
        loss_streak,
    };
    database::warps_stats::special::set(&stat, pool).await?;

    Ok(())
}

async fn calculate_stats_lc(uid: i32, pool: &PgPool) -> anyhow::Result<()> {
    let warps = database::warps::lc::get_infos_by_uid(uid, pool).await?;

    let mut pull_4 = 0;
    let mut sum_4 = 0;
    let mut count_4 = 0;

    let mut pull_5 = 0;
    let mut sum_5 = 0;
    let mut count_5 = 0;

    let mut guarantee = false;

    let mut sum_win = 0;
    let mut count_win = 0;

    let mut win_streak = 0;
    let mut max_win_streak = 0;

    let mut loss_streak = 0;
    let mut max_loss_streak = 0;

    for warp in &warps {
        pull_4 += 1;
        pull_5 += 1;

        match warp.rarity.unwrap() {
            4 => {
                count_4 += 1;
                sum_4 += pull_4;
                pull_4 = 0;
            }
            5 => {
                count_5 += 1;
                sum_5 += pull_5;
                pull_5 = 0;

                if guarantee {
                    guarantee = false;
                } else {
                    count_win += 1;

                    if [23000, 23002, 23003, 23004, 23005, 23012, 23013]
                        .contains(&warp.light_cone.unwrap())
                    {
                        win_streak = 0;

                        loss_streak += 1;
                        max_loss_streak = max_loss_streak.max(loss_streak);

                        guarantee = true;
                    } else {
                        sum_win += 1;

                        loss_streak = 0;

                        win_streak += 1;
                        max_win_streak = max_win_streak.max(win_streak);
                    }
                }
            }
            _ => {}
        }
    }

    let win_streak = max_win_streak;
    let loss_streak = max_loss_streak;

    let luck_4 = sum_4 as f64 / count_4 as f64;
    let luck_5 = sum_5 as f64 / count_5 as f64;
    let win_rate = sum_win as f64 / count_win as f64;

    let stat = database::warps_stats::lc::DbWarpsStatLc {
        uid,
        luck_4,
        luck_5,
        win_rate,
        win_streak,
        loss_streak,
    };
    database::warps_stats::lc::set(&stat, pool).await?;

    Ok(())
}
