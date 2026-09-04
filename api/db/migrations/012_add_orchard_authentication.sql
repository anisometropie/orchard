ALTER TABLE users
    ADD COLUMN password_hash TEXT;

CREATE TABLE user_sessions (
    token_hash BYTEA PRIMARY KEY CHECK (octet_length(token_hash) = 32),
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at TIMESTAMPTZ NOT NULL,
    CHECK (expires_at > created_at)
);

CREATE INDEX user_sessions_user_id_idx ON user_sessions (user_id);
CREATE INDEX user_sessions_expires_at_idx ON user_sessions (expires_at);

ALTER TABLE orchards
    ADD COLUMN share_token_hash BYTEA UNIQUE
        CHECK (share_token_hash IS NULL OR octet_length(share_token_hash) = 32);
