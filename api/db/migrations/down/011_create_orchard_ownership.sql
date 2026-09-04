DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM orchards) THEN
        RAISE EXCEPTION 'cannot remove orchard ownership while orchards exist';
    END IF;
END $$;

DROP INDEX aerial_overlays_orchard_id_idx;

ALTER TABLE aerial_overlays
    DROP COLUMN orchard_id;

ALTER TABLE plant_harvest_windows
    DROP CONSTRAINT plant_harvest_windows_unique_window,
    DROP COLUMN orchard_id,
    ADD CONSTRAINT plant_harvest_windows_unique_window UNIQUE NULLS NOT DISTINCT (
        plant_identity_id, cultivar_id,
        start_month, start_day, end_month, end_day
    );

DROP INDEX trees_orchard_id_idx;

ALTER TABLE trees
    DROP COLUMN orchard_id;

DROP TABLE orchards;
