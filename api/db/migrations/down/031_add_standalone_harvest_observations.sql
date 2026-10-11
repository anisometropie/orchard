-- SET NOT NULL refuses rollback while standalone observations exist.
ALTER TABLE harvest_observations
    ALTER COLUMN harvest_run_id SET NOT NULL,
    ALTER COLUMN window_start SET NOT NULL,
    ALTER COLUMN window_end SET NOT NULL,
    DROP CONSTRAINT harvest_observations_period_pair_check,
    DROP CONSTRAINT harvest_observations_tour_period_check;
