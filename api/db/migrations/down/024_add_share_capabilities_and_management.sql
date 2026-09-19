DO $$
BEGIN
    IF EXISTS (
        SELECT 1
        FROM orchard_share_tokens
        WHERE can_add_photos OR (can_harvest AND NOT can_water)
    ) THEN
        RAISE EXCEPTION
            'cannot remove share capabilities while tokens use permissions unsupported by the old schema';
    END IF;
END
$$;

ALTER TABLE orchard_share_tokens
    ADD COLUMN permission TEXT;

UPDATE orchard_share_tokens
SET permission = CASE
    WHEN can_harvest THEN 'harvest_watering'
    WHEN can_water THEN 'watering'
    ELSE 'view'
END;

ALTER TABLE orchard_share_tokens
    ALTER COLUMN permission SET NOT NULL,
    ADD CONSTRAINT orchard_share_tokens_permission_check
        CHECK (permission IN ('view', 'watering', 'harvest_watering')),
    DROP CONSTRAINT orchard_share_tokens_id_key,
    DROP COLUMN id,
    DROP COLUMN can_harvest,
    DROP COLUMN can_water,
    DROP COLUMN can_add_photos;

DROP INDEX orchard_share_tokens_orchard_idx;

CREATE INDEX orchard_share_tokens_orchard_permission_idx
    ON orchard_share_tokens (orchard_id, permission);
