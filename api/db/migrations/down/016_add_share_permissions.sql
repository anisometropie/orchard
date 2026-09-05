DO $$
BEGIN
    IF EXISTS (
        SELECT 1 FROM orchard_share_tokens WHERE permission = 'watering'
    ) THEN
        RAISE EXCEPTION 'cannot remove share permissions while watering links exist';
    END IF;
END $$;

ALTER TABLE orchards
    ADD COLUMN share_token_hash BYTEA UNIQUE
        CHECK (share_token_hash IS NULL OR octet_length(share_token_hash) = 32);

UPDATE orchards
SET share_token_hash = share.token_hash
FROM orchard_share_tokens share
WHERE share.orchard_id = orchards.id
  AND share.permission = 'view';

DROP TABLE orchard_share_tokens;
