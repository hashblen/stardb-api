use anyhow::Result;
use sqlx::PgPool;

pub struct DbWishesStatGlobalWeapon {
    pub uid: i32,
    pub count_percentile: f64,
    pub luck_4_percentile: f64,
    pub luck_5_percentile: f64,
}

pub async fn set(stat: &DbWishesStatGlobalWeapon, pool: &PgPool) -> Result<()> {
    sqlx::query_file!(
        "sql/gi/wishes_stats_global/weapon/set.sql",
        stat.uid,
        stat.count_percentile,
        stat.luck_4_percentile,
        stat.luck_5_percentile,
    )
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn get_by_uid(uid: i32, pool: &PgPool) -> Result<Option<DbWishesStatGlobalWeapon>> {
    Ok(sqlx::query_file_as!(
        DbWishesStatGlobalWeapon,
        "sql/gi/wishes_stats_global/weapon/get_by_uid.sql",
        uid
    )
    .fetch_optional(pool)
    .await?)
}
