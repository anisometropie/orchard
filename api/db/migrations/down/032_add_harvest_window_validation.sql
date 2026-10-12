DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM plant_harvest_windows WHERE validated) THEN
        RAISE EXCEPTION 'Cannot remove harvest window validation while validated windows exist';
    END IF;
END $$;
ALTER TABLE plant_harvest_windows DROP COLUMN validated;
