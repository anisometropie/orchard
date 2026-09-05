ALTER TABLE watering_runs
    ADD COLUMN target_kind TEXT NOT NULL DEFAULT 'row',
    ALTER COLUMN row_name DROP NOT NULL,
    ADD CONSTRAINT watering_runs_target_check CHECK (
        (target_kind = 'row' AND row_name IS NOT NULL AND btrim(row_name) <> '')
        OR (target_kind = 'danger' AND row_name IS NULL)
    );
