DO $$
BEGIN
    IF EXISTS (
        SELECT 1
        FROM watering_runs
        WHERE target_kind = 'danger'
          AND carry_capacity IS DISTINCT FROM 2
    ) THEN
        RAISE EXCEPTION 'cannot remove watering carry capacity while non-default watering history exists';
    END IF;
END $$;

ALTER TABLE watering_runs
    DROP CONSTRAINT watering_runs_carry_capacity_check,
    DROP COLUMN carry_capacity;
