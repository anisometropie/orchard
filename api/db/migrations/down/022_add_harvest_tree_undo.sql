DO $$
BEGIN
    IF EXISTS (
        SELECT 1
        FROM harvest_run_trees
        WHERE outcome = 'done_for_window'
    ) THEN
        RAISE EXCEPTION 'cannot remove done-for-window harvest history';
    END IF;
END $$;

ALTER TABLE harvest_run_trees
    DROP CONSTRAINT harvest_run_trees_outcome_check,
    DROP CONSTRAINT harvest_run_trees_resolution_check,
    ADD CONSTRAINT harvest_run_trees_outcome_check
        CHECK (outcome IN ('harvested', 'deferred')),
    ADD CONSTRAINT harvest_run_trees_resolution_check CHECK (
        (outcome IS NULL AND resolved_on IS NULL AND retry_on IS NULL)
        OR (
            outcome IS NOT NULL
            AND resolved_on IS NOT NULL
            AND (
                (outcome = 'harvested' AND retry_on IS NULL)
                OR (
                    outcome = 'deferred'
                    AND retry_on IS NOT NULL
                    AND retry_on > resolved_on
                )
            )
        )
    );

ALTER TABLE harvest_runs
    DROP COLUMN last_action_undo;
