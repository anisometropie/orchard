ALTER TABLE harvest_runs
    ADD COLUMN last_action_undo JSONB;

ALTER TABLE harvest_run_trees
    DROP CONSTRAINT harvest_run_trees_outcome_check,
    DROP CONSTRAINT harvest_run_trees_resolution_check,
    ADD CONSTRAINT harvest_run_trees_outcome_check
        CHECK (outcome IN ('harvested', 'deferred', 'done_for_window')),
    ADD CONSTRAINT harvest_run_trees_resolution_check CHECK (
        (outcome IS NULL AND resolved_on IS NULL AND retry_on IS NULL)
        OR (
            outcome IS NOT NULL
            AND resolved_on IS NOT NULL
            AND (
                (outcome IN ('harvested', 'done_for_window') AND retry_on IS NULL)
                OR (
                    outcome = 'deferred'
                    AND retry_on IS NOT NULL
                    AND retry_on > resolved_on
                )
            )
        )
    );
