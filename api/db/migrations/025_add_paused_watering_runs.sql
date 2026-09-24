ALTER TABLE watering_runs ADD COLUMN paused BOOLEAN NOT NULL DEFAULT FALSE;

DROP INDEX watering_runs_one_active_per_orchard_idx;
CREATE UNIQUE INDEX watering_runs_one_active_per_orchard_idx
    ON watering_runs (orchard_id)
    WHERE completed_at IS NULL AND NOT paused;
