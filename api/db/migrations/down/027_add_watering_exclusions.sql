DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM trees WHERE is_excluded_from_watering) THEN
        RAISE EXCEPTION 'Cannot remove watering exclusions while excluded trees exist';
    END IF;
END $$;

ALTER TABLE trees DROP COLUMN is_excluded_from_watering;
