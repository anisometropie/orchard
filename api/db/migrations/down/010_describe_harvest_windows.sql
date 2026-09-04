DO $$
BEGIN
    IF EXISTS (
        SELECT 1
        FROM plant_harvest_windows
        WHERE reference_region IS NOT NULL
           OR harvested_part <> 'fruit'
           OR data_origin <> 'external_reference'
           OR source_url IS NOT NULL
    ) THEN
        RAISE EXCEPTION 'cannot remove harvest-window descriptions while they contain non-default data';
    END IF;
END $$;

ALTER TABLE plant_harvest_windows
    DROP COLUMN reference_region,
    DROP COLUMN harvested_part,
    DROP COLUMN data_origin,
    DROP COLUMN source_url;

DROP TYPE harvest_data_origin;
DROP TYPE harvested_part;
