DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM watering_runs WHERE water_source IS NOT NULL) THEN
        RAISE EXCEPTION 'cannot remove watering sources while watering source history exists';
    END IF;
END $$;

ALTER TABLE watering_runs
    DROP COLUMN water_source;
