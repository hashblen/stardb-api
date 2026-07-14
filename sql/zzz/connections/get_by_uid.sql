SELECT
    *
FROM
    zzz_connections
WHERE
    uid = $1
    AND active = TRUE;

