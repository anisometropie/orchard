DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM watering_runs WHERE target_kind = 'danger') THEN
        RAISE EXCEPTION 'cannot remove danger watering while danger watering history exists';
    END IF;
END $$;

ALTER TABLE watering_runs
    DROP CONSTRAINT watering_runs_target_check,
    ALTER COLUMN row_name SET NOT NULL,
    DROP COLUMN target_kind;
