DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM watering_runs)
        OR EXISTS (SELECT 1 FROM trees WHERE row_rank IS NOT NULL)
    THEN
        RAISE EXCEPTION 'cannot remove row ordering and watering while saved ordering or watering history exists';
    END IF;
END $$;

DROP TABLE watering_run_trees;
DROP INDEX watering_runs_one_active_per_orchard_idx;
DROP TABLE watering_runs;

DROP INDEX trees_orchard_row_rank_key;
ALTER TABLE trees
    DROP CONSTRAINT trees_row_rank_check,
    DROP COLUMN row_rank;
