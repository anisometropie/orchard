DROP INDEX watering_runs_one_active_per_orchard_idx;
CREATE UNIQUE INDEX watering_runs_one_active_per_target_idx
    ON watering_runs (orchard_id, target_kind, COALESCE(row_name, ''))
    WHERE completed_at IS NULL AND NOT paused;
