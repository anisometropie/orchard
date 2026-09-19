DO $$
BEGIN
    IF EXISTS (
        SELECT 1
        FROM orchard_share_tokens
        WHERE permission = 'harvest_watering'
    ) THEN
        RAISE EXCEPTION
            'cannot remove harvest-and-watering sharing while combined share links exist';
    END IF;
END
$$;

ALTER TABLE orchard_share_tokens
    DROP CONSTRAINT orchard_share_tokens_permission_check,
    ADD CONSTRAINT orchard_share_tokens_permission_check
        CHECK (permission IN ('view', 'watering'));
