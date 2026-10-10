-- Refuse rollback when it would erase visit facts or production history.
DO $$ BEGIN
    IF EXISTS (SELECT 1 FROM harvest_observations WHERE kind NOT IN ('too_soon','too_late')) THEN
        RAISE EXCEPTION 'Cannot revert harvest visit facts while new visit records exist';
    END IF;
    IF EXISTS (SELECT 1 FROM harvest_run_trees WHERE outcome IN ('empty_on_arrival','fruit_remaining')) THEN
        RAISE EXCEPTION 'Cannot revert harvest visit facts while new tour outcomes exist';
    END IF;
END $$;
ALTER TABLE harvest_observations DROP CONSTRAINT harvest_observations_fruit_state_check;
ALTER TABLE harvest_observations DROP COLUMN fruit_ready, DROP COLUMN fruit_unripe, DROP COLUMN fruit_wasted;
ALTER TABLE harvest_observations DROP CONSTRAINT harvest_observations_kind_check;
ALTER TABLE harvest_observations ADD CONSTRAINT harvest_observations_kind_check CHECK (kind IN ('too_soon','too_late'));
ALTER TABLE harvest_run_trees DROP CONSTRAINT harvest_run_trees_outcome_check;
ALTER TABLE harvest_run_trees ADD CONSTRAINT harvest_run_trees_outcome_check CHECK (outcome IN ('harvested','deferred','done_for_window','too_soon','too_late'));
ALTER TABLE harvest_run_trees DROP CONSTRAINT harvest_run_trees_resolution_check;
ALTER TABLE harvest_run_trees ADD CONSTRAINT harvest_run_trees_resolution_check CHECK (
    (outcome IS NULL AND resolved_on IS NULL AND retry_on IS NULL)
    OR (outcome IS NOT NULL AND resolved_on IS NOT NULL AND (
        (outcome IN ('harvested','done_for_window','too_late') AND retry_on IS NULL)
        OR (outcome IN ('deferred','too_soon') AND retry_on IS NOT NULL AND retry_on > resolved_on)
    ))
);
