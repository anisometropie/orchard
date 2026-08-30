BEGIN;

CREATE TABLE plant_harvest_windows (
    id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    plant_identity_id BIGINT NOT NULL REFERENCES plant_identities(id) ON DELETE CASCADE,
    cultivar_id BIGINT,
    start_month SMALLINT NOT NULL,
    start_day SMALLINT NOT NULL,
    end_month SMALLINT NOT NULL,
    end_day SMALLINT NOT NULL,
    CONSTRAINT plant_harvest_windows_cultivar_matches_identity_fkey
        FOREIGN KEY (cultivar_id, plant_identity_id)
        REFERENCES plant_cultivars(id, plant_identity_id)
        ON DELETE CASCADE,
    CONSTRAINT plant_harvest_windows_start_check CHECK (
        start_month BETWEEN 1 AND 12
        AND start_day BETWEEN 1 AND CASE start_month
            WHEN 2 THEN 29
            WHEN 4 THEN 30
            WHEN 6 THEN 30
            WHEN 9 THEN 30
            WHEN 11 THEN 30
            ELSE 31
        END
    ),
    CONSTRAINT plant_harvest_windows_end_check CHECK (
        end_month BETWEEN 1 AND 12
        AND end_day BETWEEN 1 AND CASE end_month
            WHEN 2 THEN 29
            WHEN 4 THEN 30
            WHEN 6 THEN 30
            WHEN 9 THEN 30
            WHEN 11 THEN 30
            ELSE 31
        END
    ),
    CONSTRAINT plant_harvest_windows_unique_window UNIQUE NULLS NOT DISTINCT (
        plant_identity_id, cultivar_id, start_month, start_day, end_month, end_day
    )
);

CREATE INDEX plant_harvest_windows_owner_idx
    ON plant_harvest_windows (plant_identity_id, cultivar_id);

-- Migration 007 stored the former tree-level value on the botanical identity.
-- Preserve it only for trees without a known cultivar; applying it to every
-- known cultivar would repeat the assumption corrected by this migration.
INSERT INTO plant_harvest_windows (
    plant_identity_id, cultivar_id,
    start_month, start_day, end_month, end_day
)
SELECT
    id, NULL,
    harvest_start_month, harvest_start_day, harvest_end_month, harvest_end_day
FROM plant_identities
WHERE harvest_start_month IS NOT NULL;

ALTER TABLE plant_identities
    DROP CONSTRAINT plant_identities_harvest_window_completeness_check,
    DROP CONSTRAINT plant_identities_harvest_start_check,
    DROP CONSTRAINT plant_identities_harvest_end_check,
    DROP COLUMN harvest_start_month,
    DROP COLUMN harvest_start_day,
    DROP COLUMN harvest_end_month,
    DROP COLUMN harvest_end_day;

COMMIT;
