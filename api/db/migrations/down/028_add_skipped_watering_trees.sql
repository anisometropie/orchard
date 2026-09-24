DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM watering_run_trees WHERE skipped_at IS NOT NULL) THEN
        RAISE EXCEPTION 'Cannot remove watering skips while skipped tree history exists';
    END IF;
END $$;

ALTER TABLE watering_run_trees
    DROP CONSTRAINT watering_run_trees_one_outcome,
    DROP COLUMN skipped_at;
