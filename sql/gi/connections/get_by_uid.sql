SELECT
    *
FROM
    gi_connections
WHERE
    uid = $1
    AND active = TRUE;

