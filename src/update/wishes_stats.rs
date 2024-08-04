use std::time::{Duration, Instant};

use actix_web::rt;
use anyhow::Result;
use sqlx::PgPool;

use crate::database;

pub async fn spawn(pool: PgPool) {
    actix::Arbiter::new().spawn(async move {
        let mut success = true;

        let mut interval = rt::time::interval(Duration::from_secs(60 * 60 * 24));

        loop {
            if success {
                interval.tick().await;
            }

            let start = Instant::now();

            if let Err(e) = update(pool.clone()).await {
                error!(
                    "Wishes stats update failed with {e} in {}s",
                    start.elapsed().as_secs_f64()
                );

                success = false;
            } else {
                info!(
                    "Wishes stats update succeeded in {}s",
                    start.elapsed().as_secs_f64()
                );

                success = true;
            }
        }
    });
}

async fn update(pool: PgPool) -> Result<()> {
    let uids = database::gi::wishes::get_uids(&pool).await?;
    //
    //info!("Starting standard");
    //standard(&uids, &pool).await?;
    //
    //info!("Starting character");
    //character(&uids, &pool).await?;
    //
    //info!("Starting lc");
    //lc(&uids, &pool).await?;

    for uid in uids {
        calculate_stats_standard(uid, &pool).await?;
        calculate_stats_character(uid, &pool).await?;
        calculate_stats_weapon(uid, &pool).await?;
        calculate_stats_chronicled(uid, &pool).await?;
    }

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

//async fn standard(uids: &[i32], pool: &PgPool) -> Result<()> {
//    let mut count_map = HashMap::new();
//    let mut luck_4_map = HashMap::new();
//    let mut luck_5_map = HashMap::new();
//
//    let mut stat_uids = Vec::new();
//
//    for &uid in uids {
//        let warp_stat = database::gi::wishes_stats::standard::get_by_uid(uid, pool).await?;
//
//        let count = database::gi::wishes::standard::get_count_by_uid(uid, pool).await? as i32;
//
//        if count < 100 {
//            continue;
//        }
//
//        stat_uids.push(uid);
//
//        count_map.insert(uid, count);
//        luck_4_map.insert(uid, warp_stat.luck_4);
//        luck_5_map.insert(uid, warp_stat.luck_5);
//    }
//
//    let mut sorted_count: Vec<(i32, i32)> = count_map.iter().map(|(&k, &v)| (k, v)).collect();
//    sorted_count.sort_unstable_by(|(_, v1), (_, v2)| v2.cmp(v1));
//
//    let mut sorted_luck_4: Vec<(i32, f64)> = luck_4_map.iter().map(|(&k, &v)| (k, v)).collect();
//    sorted_luck_4.sort_unstable_by(|(_, v1), (_, v2)| v1.partial_cmp(v2).unwrap());
//
//    let mut sorted_luck_5: Vec<(i32, f64)> = luck_5_map.iter().map(|(&k, &v)| (k, v)).collect();
//    sorted_luck_5.sort_unstable_by(|(_, v1), (_, v2)| v1.partial_cmp(v2).unwrap());
//
//    let count_percentiles: HashMap<_, _> = sorted_count
//        .into_iter()
//        .enumerate()
//        .map(|(i, (uid, _))| (uid, i))
//        .collect();
//
//    let luck_4_percentiles: HashMap<_, _> = sorted_luck_4
//        .into_iter()
//        .enumerate()
//        .map(|(i, (uid, _))| (uid, i))
//        .collect();
//
//    let luck_5_percentiles: HashMap<_, _> = sorted_luck_5
//        .into_iter()
//        .enumerate()
//        .map(|(i, (uid, _))| (uid, i))
//        .collect();
//
//    let len = stat_uids.len() as f64;
//    for uid in &stat_uids {
//        let count_percentile = count_percentiles[uid] as f64 / len;
//        let luck_4_percentile = luck_4_percentiles[uid] as f64 / len;
//        let luck_5_percentile = luck_5_percentiles[uid] as f64 / len;
//
//        let stat = database::wishes_stats_global::standard::DbWishesStatGlobalStandard {
//            uid: *uid,
//            count_percentile,
//            luck_4_percentile,
//            luck_5_percentile,
//        };
//
//        database::wishes_stats_global::standard::set(&stat, pool).await?;
//    }
//
//    Ok(())
//}
//
//async fn character(uids: &[i32], pool: &PgPool) -> Result<()> {
//    let mut count_map = HashMap::new();
//    let mut luck_4_map = HashMap::new();
//    let mut luck_5_map = HashMap::new();
//
//    let mut stat_uids = Vec::new();
//
//    for &uid in uids {
//        let warp_stat = database::wishes_stats::character::get_by_uid(uid, pool).await?;
//
//        let count = database::wishes::character::get_count_by_uid(uid, pool).await? as i32;
//
//        if count < 100 {
//            continue;
//        }
//
//        stat_uids.push(uid);
//
//        count_map.insert(uid, count);
//        luck_4_map.insert(uid, warp_stat.luck_4);
//        luck_5_map.insert(uid, warp_stat.luck_5);
//    }
//
//    let mut sorted_count: Vec<(i32, i32)> = count_map.iter().map(|(&k, &v)| (k, v)).collect();
//    sorted_count.sort_unstable_by(|(_, v1), (_, v2)| v2.cmp(v1));
//
//    let mut sorted_luck_4: Vec<(i32, f64)> = luck_4_map.iter().map(|(&k, &v)| (k, v)).collect();
//    sorted_luck_4.sort_unstable_by(|(_, v1), (_, v2)| v1.partial_cmp(v2).unwrap());
//
//    let mut sorted_luck_5: Vec<(i32, f64)> = luck_5_map.iter().map(|(&k, &v)| (k, v)).collect();
//    sorted_luck_5.sort_unstable_by(|(_, v1), (_, v2)| v1.partial_cmp(v2).unwrap());
//
//    let count_percentiles: HashMap<_, _> = sorted_count
//        .into_iter()
//        .enumerate()
//        .map(|(i, (uid, _))| (uid, i))
//        .collect();
//
//    let luck_4_percentiles: HashMap<_, _> = sorted_luck_4
//        .into_iter()
//        .enumerate()
//        .map(|(i, (uid, _))| (uid, i))
//        .collect();
//
//    let luck_5_percentiles: HashMap<_, _> = sorted_luck_5
//        .into_iter()
//        .enumerate()
//        .map(|(i, (uid, _))| (uid, i))
//        .collect();
//
//    let len = stat_uids.len() as f64;
//    for uid in &stat_uids {
//        let count_percentile = count_percentiles[uid] as f64 / len;
//        let luck_4_percentile = luck_4_percentiles[uid] as f64 / len;
//        let luck_5_percentile = luck_5_percentiles[uid] as f64 / len;
//
//        let stat = database::wishes_stats_global::character::DbWishesStatGlobalCharacter {
//            uid: *uid,
//            count_percentile,
//            luck_4_percentile,
//            luck_5_percentile,
//        };
//
//        database::wishes_stats_global::character::set(&stat, pool).await?;
//    }
//
//    Ok(())
//}
//
//async fn lc(uids: &[i32], pool: &PgPool) -> Result<()> {
//    let mut count_map = HashMap::new();
//    let mut luck_4_map = HashMap::new();
//    let mut luck_5_map = HashMap::new();
//
//    let mut stat_uids = Vec::new();
//
//    for &uid in uids {
//        let warp_stat = database::wishes_stats::lc::get_by_uid(uid, pool).await?;
//
//        let count = database::wishes::lc::get_count_by_uid(uid, pool).await? as i32;
//
//        if count < 100 {
//            continue;
//        }
//
//        stat_uids.push(uid);
//
//        count_map.insert(uid, count);
//        luck_4_map.insert(uid, warp_stat.luck_4);
//        luck_5_map.insert(uid, warp_stat.luck_5);
//    }
//
//    let mut sorted_count: Vec<(i32, i32)> = count_map.iter().map(|(&k, &v)| (k, v)).collect();
//    sorted_count.sort_unstable_by(|(_, v1), (_, v2)| v2.cmp(v1));
//
//    let mut sorted_luck_4: Vec<(i32, f64)> = luck_4_map.iter().map(|(&k, &v)| (k, v)).collect();
//    sorted_luck_4.sort_unstable_by(|(_, v1), (_, v2)| v1.partial_cmp(v2).unwrap());
//
//    let mut sorted_luck_5: Vec<(i32, f64)> = luck_5_map.iter().map(|(&k, &v)| (k, v)).collect();
//    sorted_luck_5.sort_unstable_by(|(_, v1), (_, v2)| v1.partial_cmp(v2).unwrap());
//
//    let count_percentiles: HashMap<_, _> = sorted_count
//        .into_iter()
//        .enumerate()
//        .map(|(i, (uid, _))| (uid, i))
//        .collect();
//
//    let luck_4_percentiles: HashMap<_, _> = sorted_luck_4
//        .into_iter()
//        .enumerate()
//        .map(|(i, (uid, _))| (uid, i))
//        .collect();
//
//    let luck_5_percentiles: HashMap<_, _> = sorted_luck_5
//        .into_iter()
//        .enumerate()
//        .map(|(i, (uid, _))| (uid, i))
//        .collect();
//
//    let len = stat_uids.len() as f64;
//    for uid in &stat_uids {
//        let count_percentile = count_percentiles[uid] as f64 / len;
//        let luck_4_percentile = luck_4_percentiles[uid] as f64 / len;
//        let luck_5_percentile = luck_5_percentiles[uid] as f64 / len;
//
//        let stat = database::wishes_stats_global::lc::DbWishesStatGlobalLc {
//            uid: *uid,
//            count_percentile,
//            luck_4_percentile,
//            luck_5_percentile,
//        };
//
//        database::wishes_stats_global::lc::set(&stat, pool).await?;
//    }
//
//    Ok(())
//}
