DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM users WHERE password_hash IS NOT NULL)
        OR EXISTS (SELECT 1 FROM user_sessions)
        OR EXISTS (SELECT 1 FROM orchards WHERE share_token_hash IS NOT NULL)
    THEN
        RAISE EXCEPTION 'cannot remove orchard authentication while credentials or tokens exist';
    END IF;
END $$;

ALTER TABLE orchards
    DROP COLUMN share_token_hash;

DROP INDEX user_sessions_expires_at_idx;
DROP INDEX user_sessions_user_id_idx;
DROP TABLE user_sessions;

ALTER TABLE users
    DROP COLUMN password_hash;
