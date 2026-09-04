DO $$
BEGIN
    IF EXISTS (
        SELECT 1
        FROM plant_harvest_windows
        WHERE cultivar_id IS NOT NULL
    ) THEN
        RAISE EXCEPTION 'cannot collapse cultivar-specific harvest windows into migration 007';
    END IF;
END $$;

ALTER TABLE plant_identities
    ADD COLUMN harvest_start_month SMALLINT,
    ADD COLUMN harvest_start_day SMALLINT,
    ADD COLUMN harvest_end_month SMALLINT,
    ADD COLUMN harvest_end_day SMALLINT;

UPDATE plant_identities identity
SET
    harvest_start_month = harvest_window.start_month,
    harvest_start_day = harvest_window.start_day,
    harvest_end_month = harvest_window.end_month,
    harvest_end_day = harvest_window.end_day
FROM plant_harvest_windows harvest_window
WHERE harvest_window.plant_identity_id = identity.id
  AND harvest_window.cultivar_id IS NULL;

ALTER TABLE plant_identities
    ADD CONSTRAINT plant_identities_harvest_window_completeness_check CHECK (
        (
            harvest_start_month IS NULL
            AND harvest_start_day IS NULL
            AND harvest_end_month IS NULL
            AND harvest_end_day IS NULL
        )
        OR
        (
            harvest_start_month IS NOT NULL
            AND harvest_start_day IS NOT NULL
            AND harvest_end_month IS NOT NULL
            AND harvest_end_day IS NOT NULL
        )
    ),
    ADD CONSTRAINT plant_identities_harvest_start_check CHECK (
        harvest_start_month IS NULL
        OR (
            harvest_start_month BETWEEN 1 AND 12
            AND harvest_start_day BETWEEN 1 AND CASE harvest_start_month
                WHEN 2 THEN 29
                WHEN 4 THEN 30
                WHEN 6 THEN 30
                WHEN 9 THEN 30
                WHEN 11 THEN 30
                ELSE 31
            END
        )
    ),
    ADD CONSTRAINT plant_identities_harvest_end_check CHECK (
        harvest_end_month IS NULL
        OR (
            harvest_end_month BETWEEN 1 AND 12
            AND harvest_end_day BETWEEN 1 AND CASE harvest_end_month
                WHEN 2 THEN 29
                WHEN 4 THEN 30
                WHEN 6 THEN 30
                WHEN 9 THEN 30
                WHEN 11 THEN 30
                ELSE 31
            END
        )
    );

DROP TABLE plant_harvest_windows;
