-- Preserve tour progress by mapping new reasons back to the old supported outcomes.
UPDATE harvest_runs SET last_action_undo = NULL
WHERE last_action_undo ? 'observation_id'
   OR last_action_undo::text LIKE '%too_soon%'
   OR last_action_undo::text LIKE '%too_late%';
ALTER TABLE harvest_run_trees DROP CONSTRAINT harvest_run_trees_outcome_check;
ALTER TABLE harvest_run_trees DROP CONSTRAINT harvest_run_trees_resolution_check;
UPDATE harvest_run_trees SET outcome = 'done_for_window', retry_on = NULL
WHERE outcome IN ('too_soon', 'too_late');
ALTER TABLE harvest_run_trees ADD CONSTRAINT harvest_run_trees_outcome_check
CHECK (outcome IN ('harvested', 'deferred', 'done_for_window'));
ALTER TABLE harvest_run_trees ADD CONSTRAINT harvest_run_trees_resolution_check CHECK (
    (outcome IS NULL AND resolved_on IS NULL AND retry_on IS NULL)
    OR (outcome IS NOT NULL AND resolved_on IS NOT NULL AND (
        (outcome IN ('harvested','done_for_window') AND retry_on IS NULL)
        OR (outcome = 'deferred' AND retry_on > resolved_on AND retry_on IS NOT NULL)
    ))
);
DROP TABLE harvest_observations;
