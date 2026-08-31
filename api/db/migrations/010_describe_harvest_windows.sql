BEGIN;

CREATE TYPE harvested_part AS ENUM (
    'cone',
    'flower',
    'fruit',
    'leaf',
    'nut',
    'pod',
    'seed'
);

CREATE TYPE harvest_data_origin AS ENUM (
    'external_reference',
    'field_observation'
);

ALTER TABLE plant_harvest_windows
    ADD COLUMN reference_region TEXT,
    ADD COLUMN harvested_part harvested_part NOT NULL DEFAULT 'fruit',
    ADD COLUMN data_origin harvest_data_origin NOT NULL DEFAULT 'external_reference',
    ADD COLUMN source_url TEXT;

ALTER TABLE plant_harvest_windows
    ALTER COLUMN harvested_part DROP DEFAULT,
    ALTER COLUMN data_origin DROP DEFAULT;

COMMIT;
