DO $$
BEGIN
    IF EXISTS (
        SELECT orchard_id, permission
        FROM orchard_share_tokens
        GROUP BY orchard_id, permission
        HAVING count(*) > 1
    ) THEN
        RAISE EXCEPTION 'cannot restore rotating share links while multiple links exist for an orchard permission';
    END IF;
END $$;

DROP INDEX orchard_share_tokens_orchard_permission_idx;

ALTER TABLE orchard_share_tokens
    DROP CONSTRAINT orchard_share_tokens_pkey,
    ADD CONSTRAINT orchard_share_tokens_token_hash_key UNIQUE (token_hash),
    ADD PRIMARY KEY (orchard_id, permission);
