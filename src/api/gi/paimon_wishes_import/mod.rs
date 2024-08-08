use actix_session::Session;
use actix_web::{post, web, HttpResponse, Responder};
use chrono::NaiveDateTime;
use sqlx::PgPool;
use utoipa::OpenApi;

use crate::{api::ApiResult, database, GiGachaType};

#[derive(OpenApi)]
#[openapi(
    tags((name = "paimon-wishes-import")),
    paths(post_paimon_warps_import),
    components(schemas(PaimonWishesImportParams)),
)]
struct ApiDoc;

pub fn openapi() -> utoipa::openapi::OpenApi {
    ApiDoc::openapi()
}

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(post_paimon_warps_import);
}

#[derive(serde::Deserialize, utoipa::ToSchema)]
struct PaimonWishesImportParams {
    data: String,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
struct Paimon {
    wish_uid: serde_json::Value,
    wish_counter_beginners: Option<Wishes>,
    wish_counter_standard: Option<Wishes>,
    wish_counter_character_event: Option<Wishes>,
    wish_counter_weapon_event: Option<Wishes>,
    wish_counter_chronicled: Option<Wishes>,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
struct Wishes {
    pulls: Vec<Pull>,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
struct Pull {
    r#type: String,
    id: String,
    time: String,
}

#[utoipa::path(
    tag = "paimon-wishes-import",
    post,
    path = "/api/paimon-wishes-import",
    request_body = PaimonWishesImportParams,
    responses(
        (status = 200, description = "Warps imported"),
        (status = 403, description = "Not an admin"),
    ),
    security(("admin" = []))
)]
#[post("/api/paimon-wishes-import")]
async fn post_paimon_warps_import(
    session: Session,
    params: web::Json<PaimonWishesImportParams>,
    pool: web::Data<PgPool>,
) -> ApiResult<impl Responder> {
    let Ok(Some(username)) = session.get::<String>("username") else {
        return Ok(HttpResponse::BadRequest().finish());
    };

    if database::admins::get_one_by_username(&username, &pool)
        .await
        .is_err()
    {
        return Ok(HttpResponse::Forbidden().finish());
    }

    let paimon: Paimon = serde_json::from_str(&params.data)?;

    let uid = if let Some(uid) = paimon.wish_uid.as_i64() {
        uid as i32
    } else {
        paimon.wish_uid.as_str().unwrap().parse()?
    };

    if database::gi::profiles::get_by_uid(uid, &pool)
        .await
        .is_err()
    {
        return Ok(HttpResponse::BadRequest().finish());
    }

    let timestamp_offset = chrono::Duration::hours(match uid.to_string().chars().next() {
        Some('6') => -5,
        Some('7') => 1,
        _ => 8,
    });

    let paimon: Paimon = serde_json::from_str(&params.data)?;

    let mut set_all_beginner = database::gi::wishes::SetAll::default();
    let mut set_all_standard = database::gi::wishes::SetAll::default();
    let mut set_all_character = database::gi::wishes::SetAll::default();
    let mut set_all_weapon = database::gi::wishes::SetAll::default();
    let mut set_all_chronicled = database::gi::wishes::SetAll::default();

    for (wishes, gacha_type) in [
        (&paimon.wish_counter_beginners, GiGachaType::Beginner),
        (&paimon.wish_counter_standard, GiGachaType::Standard),
        (&paimon.wish_counter_character_event, GiGachaType::Character),
        (&paimon.wish_counter_weapon_event, GiGachaType::Weapon),
        (&paimon.wish_counter_chronicled, GiGachaType::Chronicled),
    ] {
        let Some(wishes) = wishes else {
            continue;
        };

        let earliest_timestamp = match gacha_type {
            GiGachaType::Beginner => {
                database::gi::wishes::beginner::get_earliest_timestamp_by_uid(uid, &pool).await?
            }
            GiGachaType::Standard => {
                database::gi::wishes::standard::get_earliest_timestamp_by_uid(uid, &pool).await?
            }
            GiGachaType::Character => {
                database::gi::wishes::character::get_earliest_timestamp_by_uid(uid, &pool).await?
            }
            GiGachaType::Weapon => {
                database::gi::wishes::weapon::get_earliest_timestamp_by_uid(uid, &pool).await?
            }
            GiGachaType::Chronicled => {
                database::gi::wishes::chronicled::get_earliest_timestamp_by_uid(uid, &pool).await?
            }
        };

        for (id, wish) in wishes.pulls.iter().enumerate() {
            let (character, weapon) = match wish.r#type.as_str() {
                "character" => (
                    Some(database::gi::characters::get_id_by_paimon_moe_id(&wish.id, &pool).await?),
                    None,
                ),
                "weapon" => (
                    None,
                    Some(database::gi::weapons::get_id_by_paimon_moe_id(&wish.id, &pool).await?),
                ),
                _ => return Ok(HttpResponse::BadRequest().finish()),
            };

            let timestamp = NaiveDateTime::parse_from_str(&wish.time, "%Y-%m-%d %H:%M:%S")?
                .and_utc()
                - timestamp_offset;

            if let Some(earliest_timestamp) = earliest_timestamp {
                if timestamp >= earliest_timestamp {
                    break;
                }
            }

            let set_all = match gacha_type {
                GiGachaType::Beginner => &mut set_all_beginner,
                GiGachaType::Standard => &mut set_all_standard,
                GiGachaType::Character => &mut set_all_character,
                GiGachaType::Weapon => &mut set_all_weapon,
                GiGachaType::Chronicled => &mut set_all_chronicled,
            };

            set_all.id.push(id as i64);
            set_all.uid.push(uid);
            set_all.character.push(character);
            set_all.weapon.push(weapon);
            set_all.timestamp.push(timestamp);
            set_all.official.push(false);
        }
    }

    database::gi::wishes::beginner::set_all(&set_all_beginner, &pool).await?;
    database::gi::wishes::standard::set_all(&set_all_standard, &pool).await?;
    database::gi::wishes::character::set_all(&set_all_character, &pool).await?;
    database::gi::wishes::weapon::set_all(&set_all_weapon, &pool).await?;
    database::gi::wishes::chronicled::set_all(&set_all_chronicled, &pool).await?;

    calculate_stats(uid, &pool).await?;

    Ok(HttpResponse::Ok().finish())
}

async fn calculate_stats(uid: i32, pool: &PgPool) -> anyhow::Result<()> {
    calculate_stats_standard(uid, pool).await?;
    calculate_stats_character(uid, pool).await?;
    calculate_stats_weapon(uid, pool).await?;
    calculate_stats_chronicled(uid, pool).await?;

    Ok(())
}

async fn calculate_stats_standard(uid: i32, pool: &PgPool) -> anyhow::Result<()> {
    let wishes = database::gi::wishes::standard::get_infos_by_uid(uid, pool).await?;

    let mut pull_4 = 0;
    let mut sum_4 = 0;
    let mut count_4 = 0;

    let mut pull_5 = 0;
    let mut sum_5 = 0;
    let mut count_5 = 0;

    for wish in &wishes {
        pull_4 += 1;
        pull_5 += 1;

        match wish.rarity.unwrap() {
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

    let stat = database::gi::wishes_stats::standard::DbWishesStatStandard {
        uid,
        luck_4,
        luck_5,
    };
    database::gi::wishes_stats::standard::set(&stat, pool).await?;

    Ok(())
}

async fn calculate_stats_character(uid: i32, pool: &PgPool) -> anyhow::Result<()> {
    let wishes = database::gi::wishes::character::get_infos_by_uid(uid, pool).await?;

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

    for wish in &wishes {
        pull_4 += 1;
        pull_5 += 1;

        match wish.rarity.unwrap() {
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

                    if [
                        10000042, 10000016, 10000003, 10000035, 10000069, 10000079, 10000041,
                    ]
                    .contains(&wish.character.unwrap())
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

    let stat = database::gi::wishes_stats::character::DbWishesStatCharacter {
        uid,
        luck_4,
        luck_5,
        win_rate,
        win_streak,
        loss_streak,
    };
    database::gi::wishes_stats::character::set(&stat, pool).await?;

    Ok(())
}

async fn calculate_stats_weapon(uid: i32, pool: &PgPool) -> anyhow::Result<()> {
    let wishes = database::gi::wishes::weapon::get_infos_by_uid(uid, pool).await?;

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

    for wish in &wishes {
        pull_4 += 1;
        pull_5 += 1;

        match wish.rarity.unwrap() {
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

                    if [
                        15502, 11501, 14502, 13505, 14501, 15501, 12501, 13502, 12502,
                    ]
                    .contains(&wish.weapon.unwrap())
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

    let stat = database::gi::wishes_stats::weapon::DbWishesStatWeapon {
        uid,
        luck_4,
        luck_5,
        win_rate,
        win_streak,
        loss_streak,
    };
    database::gi::wishes_stats::weapon::set(&stat, pool).await?;

    Ok(())
}

async fn calculate_stats_chronicled(uid: i32, pool: &PgPool) -> anyhow::Result<()> {
    let wishes = database::gi::wishes::chronicled::get_infos_by_uid(uid, pool).await?;

    let mut pull_4 = 0;
    let mut sum_4 = 0;
    let mut count_4 = 0;

    let mut pull_5 = 0;
    let mut sum_5 = 0;
    let mut count_5 = 0;

    for wish in &wishes {
        pull_4 += 1;
        pull_5 += 1;

        match wish.rarity.unwrap() {
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

    let stat = database::gi::wishes_stats::chronicled::DbWishesStatChronicled {
        uid,
        luck_4,
        luck_5,
    };
    database::gi::wishes_stats::chronicled::set(&stat, pool).await?;

    Ok(())
}
