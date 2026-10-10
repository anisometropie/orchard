ALTER TABLE harvest_observations
    DROP CONSTRAINT harvest_observations_kind_check,
    ADD CONSTRAINT harvest_observations_kind_check CHECK (
        kind IN ('too_soon','too_late','harvested_all','empty_on_arrival','all_wasted','fruit_remaining')
    ),
    ADD COLUMN fruit_ready BOOLEAN,
    ADD COLUMN fruit_unripe BOOLEAN,
    ADD COLUMN fruit_wasted BOOLEAN,
    ADD CONSTRAINT harvest_observations_fruit_state_check CHECK (
        (kind IN ('too_soon','too_late') AND fruit_ready IS NULL AND fruit_unripe IS NULL AND fruit_wasted IS NULL)
        OR (fruit_ready IS NOT NULL AND fruit_unripe IS NOT NULL AND fruit_wasted IS NOT NULL AND (
            (kind = 'harvested_all' AND fruit_ready AND NOT fruit_unripe AND NOT fruit_wasted)
            OR (kind = 'empty_on_arrival' AND NOT fruit_ready AND NOT fruit_unripe AND NOT fruit_wasted)
            OR (kind = 'all_wasted' AND NOT fruit_ready AND NOT fruit_unripe AND fruit_wasted)
            OR (kind = 'fruit_remaining' AND (fruit_ready OR fruit_unripe))
        ))
    );

ALTER TABLE harvest_run_trees DROP CONSTRAINT harvest_run_trees_outcome_check;
ALTER TABLE harvest_run_trees ADD CONSTRAINT harvest_run_trees_outcome_check CHECK (
    outcome IN ('harvested','deferred','done_for_window','too_soon','too_late','empty_on_arrival','fruit_remaining')
);
ALTER TABLE harvest_run_trees DROP CONSTRAINT harvest_run_trees_resolution_check;
ALTER TABLE harvest_run_trees ADD CONSTRAINT harvest_run_trees_resolution_check CHECK (
    (outcome IS NULL AND resolved_on IS NULL AND retry_on IS NULL)
    OR (outcome IS NOT NULL AND resolved_on IS NOT NULL AND (
        (outcome IN ('harvested','done_for_window','too_late','empty_on_arrival') AND retry_on IS NULL)
        OR (outcome IN ('deferred','too_soon','fruit_remaining') AND retry_on IS NOT NULL AND retry_on > resolved_on)
    ))
);
