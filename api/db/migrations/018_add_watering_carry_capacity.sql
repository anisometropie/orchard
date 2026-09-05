ALTER TABLE watering_runs
    ADD COLUMN carry_capacity INTEGER;

UPDATE watering_runs
SET carry_capacity = 2
WHERE target_kind = 'danger';

ALTER TABLE watering_runs
    ADD CONSTRAINT watering_runs_carry_capacity_check CHECK (
        (target_kind = 'row' AND carry_capacity IS NULL)
        OR (target_kind = 'danger' AND carry_capacity > 0)
    );
