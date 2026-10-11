ALTER TABLE harvest_observations
    ALTER COLUMN harvest_run_id DROP NOT NULL,
    ALTER COLUMN window_start DROP NOT NULL,
    ALTER COLUMN window_end DROP NOT NULL,
    ADD CONSTRAINT harvest_observations_period_pair_check CHECK (
        (window_start IS NULL AND window_end IS NULL)
        OR (window_start IS NOT NULL AND window_end IS NOT NULL)
    ),
    ADD CONSTRAINT harvest_observations_tour_period_check CHECK (
        harvest_run_id IS NULL OR window_start IS NOT NULL
    );
