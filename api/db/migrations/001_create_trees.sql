CREATE EXTENSION IF NOT EXISTS postgis;

CREATE TABLE IF NOT EXISTS trees (
    id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    legacy_feature_id INTEGER UNIQUE,
    location geometry(Point, 4326) NOT NULL,
    name TEXT NOT NULL,
    latin_name TEXT,
    planted_on DATE,
    row_name TEXT,
    roles TEXT[] NOT NULL DEFAULT '{}',
    is_alive BOOLEAN NOT NULL,
    harvest_start_day SMALLINT,
    harvest_end_day SMALLINT,
    adult_height_meters DOUBLE PRECISION,
    adult_width_meters DOUBLE PRECISION,
    CHECK (harvest_start_day IS NULL OR harvest_start_day BETWEEN 1 AND 366),
    CHECK (harvest_end_day IS NULL OR harvest_end_day BETWEEN 1 AND 366)
);
