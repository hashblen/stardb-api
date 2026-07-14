UPDATE gi_connections
SET active = $3
WHERE uid = $1
    AND username = $2;