DO $$
BEGIN
    IF EXISTS (
        SELECT orchard_id FROM watering_runs
        WHERE completed_at IS NULL AND NOT paused
        GROUP BY orchard_id HAVING count(*) > 1
    ) THEN
        RAISE EXCEPTION 'Cannot remove parallel watering support while multiple active runs exist';
    END IF;
END $$;

DROP INDEX watering_runs_one_active_per_target_idx;
CREATE UNIQUE INDEX watering_runs_one_active_per_orchard_idx
    ON watering_runs (orchard_id)
    WHERE completed_at IS NULL AND NOT paused;
