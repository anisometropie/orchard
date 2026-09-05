ALTER TABLE orchard_share_tokens
    DROP CONSTRAINT orchard_share_tokens_pkey,
    DROP CONSTRAINT orchard_share_tokens_token_hash_key,
    ADD PRIMARY KEY (token_hash);

CREATE INDEX orchard_share_tokens_orchard_permission_idx
    ON orchard_share_tokens (orchard_id, permission);
