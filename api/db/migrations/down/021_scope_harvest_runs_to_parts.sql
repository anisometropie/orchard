DO $$
BEGIN
    IF EXISTS (
        SELECT 1 FROM harvest_runs
        WHERE harvested_parts <> ARRAY['fruit']::TEXT[]
    ) OR EXISTS (
        SELECT 1 FROM harvest_run_trees
        WHERE harvested_parts <> ARRAY['fruit']::TEXT[]
    ) THEN
        RAISE EXCEPTION
            'cannot revert selected harvest parts while non-fruit harvest history exists';
    END IF;
END
$$;

ALTER TABLE harvest_run_trees
    DROP CONSTRAINT harvest_run_trees_parts_check,
    DROP COLUMN harvested_parts;

ALTER TABLE harvest_runs
    DROP CONSTRAINT harvest_runs_parts_check,
    DROP COLUMN harvested_parts;
