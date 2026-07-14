use anyhow::Result;
use sqlx::PgPool;

pub struct DbConnection {
    pub uid: i32,
    pub username: String,
    pub verified: bool,
    pub private: bool,
    pub active: bool,
}

pub async fn set(connection: &DbConnection, pool: &PgPool) -> Result<()> {
    sqlx::query!(
        "INSERT INTO connections
            (uid, username, verified, private, active) 
        VALUES
            ($1, $2, $3, $4, $5) 
        ON CONFLICT
            (uid, username) 
        DO UPDATE SET 
            verified = EXCLUDED.verified,
            active = EXCLUDED.active
        ",
        connection.uid,
        connection.username,
        connection.verified,
        connection.private,
        connection.active,
    )
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn set_active(uid: i32, username: &str, active: bool, pool: &PgPool) -> Result<()> {
    sqlx::query!(
        "UPDATE connections SET active = $3 WHERE uid = $1 AND username = $2",
        uid,
        username,
        active,
    )
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn get_by_uid(uid: i32, pool: &PgPool) -> Result<Vec<DbConnection>> {
    Ok(sqlx::query_as!(
        DbConnection,
        "SELECT * FROM connections WHERE uid = $1 AND active = TRUE",
        uid,
    )
    .fetch_all(pool)
    .await?)
}

pub async fn get_by_uid_all(uid: i32, pool: &PgPool) -> Result<Vec<DbConnection>> {
    Ok(sqlx::query_as!(
        DbConnection,
        "SELECT * FROM connections WHERE uid = $1",
        uid,
    )
    .fetch_all(pool)
    .await?)
}

pub async fn get_by_username(username: &str, pool: &PgPool) -> Result<Vec<DbConnection>> {
    Ok(sqlx::query_as!(
        DbConnection,
        "SELECT * FROM connections WHERE username = $1 AND active = TRUE",
        username
    )
    .fetch_all(pool)
    .await?)
}

pub async fn get_by_uid_and_username(
    uid: i32,
    username: &str,
    pool: &PgPool,
) -> Result<DbConnection> {
    Ok(sqlx::query_as!(
        DbConnection,
        "SELECT * FROM connections WHERE uid = $1 AND username = $2 AND active = TRUE",
        uid,
        username,
    )
    .fetch_one(pool)
    .await?)
}

pub async fn update_private_by_uid_and_username(
    uid: i32,
    username: &str,
    private: bool,
    pool: &PgPool,
) -> Result<()> {
    sqlx::query!(
        "UPDATE connections SET private = $3 WHERE uid = $1 AND username = $2",
        uid,
        username,
        private,
    )
    .execute(pool)
    .await?;

    Ok(())
}
