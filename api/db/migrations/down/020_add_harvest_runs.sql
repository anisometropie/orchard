DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM harvest_runs) THEN
        RAISE EXCEPTION 'cannot remove harvest runs while harvest history exists';
    END IF;
END $$;

DROP TABLE harvest_run_trees;
DROP INDEX harvest_runs_one_active_per_orchard_idx;
DROP TABLE harvest_runs;
