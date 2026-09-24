ALTER TABLE watering_run_trees
    ADD COLUMN skipped_at TIMESTAMPTZ,
    ADD CONSTRAINT watering_run_trees_one_outcome CHECK (
        watered_at IS NULL OR skipped_at IS NULL
    );
