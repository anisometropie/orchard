ALTER TABLE orchard_share_tokens
    DROP CONSTRAINT orchard_share_tokens_permission_check,
    ADD CONSTRAINT orchard_share_tokens_permission_check
        CHECK (permission IN ('view', 'watering', 'harvest_watering'));
