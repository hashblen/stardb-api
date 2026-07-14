INSERT INTO gi_connections (uid, username, verified, private, active)
    VALUES ($1, $2, $3, $4, $5)
ON CONFLICT (uid, username)
    DO UPDATE SET
        verified = EXCLUDED.verified,
        active = EXCLUDED.active;

