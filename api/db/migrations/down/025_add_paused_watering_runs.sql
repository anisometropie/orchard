DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM watering_runs WHERE paused) THEN
        RAISE EXCEPTION 'Cannot remove pause support while paused watering runs exist';
    END IF;
END $$;

DROP INDEX watering_runs_one_active_per_orchard_idx;
CREATE UNIQUE INDEX watering_runs_one_active_per_orchard_idx
    ON watering_runs (orchard_id)
    WHERE completed_at IS NULL;
ALTER TABLE watering_runs DROP COLUMN paused;
