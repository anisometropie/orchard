CREATE TABLE orchard_share_tokens (
    orchard_id BIGINT NOT NULL REFERENCES orchards(id) ON DELETE CASCADE,
    permission TEXT NOT NULL CHECK (permission IN ('view', 'watering')),
    token_hash BYTEA NOT NULL UNIQUE CHECK (octet_length(token_hash) = 32),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (orchard_id, permission)
);

INSERT INTO orchard_share_tokens (orchard_id, permission, token_hash)
SELECT id, 'view', share_token_hash
FROM orchards
WHERE share_token_hash IS NOT NULL;

ALTER TABLE orchards
    DROP COLUMN share_token_hash;
