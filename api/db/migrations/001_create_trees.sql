CREATE EXTENSION IF NOT EXISTS postgis;

CREATE TABLE IF NOT EXISTS plant_identities (
    id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    common_name TEXT NOT NULL,
    botanical_taxon JSONB NOT NULL,
    cultivar TEXT,
    trade_name TEXT,
    identification_status TEXT NOT NULL,
    identity_key TEXT NOT NULL UNIQUE
);

CREATE TABLE IF NOT EXISTS trees (
    id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    legacy_feature_id INTEGER UNIQUE,
    plant_identity_id BIGINT NOT NULL REFERENCES plant_identities(id),
    location geometry(Point, 4326) NOT NULL,
    legacy_name TEXT,
    legacy_latin_name TEXT,
    legacy_source_url TEXT,
    legacy_identification_name TEXT,
    legacy_identification_latin_name TEXT,
    planted_on DATE,
    row_name TEXT,
    roles TEXT[] NOT NULL DEFAULT '{}',
    is_alive BOOLEAN NOT NULL,
    reproductive_role TEXT,
    harvest_start_day SMALLINT,
    harvest_end_day SMALLINT,
    adult_height_meters DOUBLE PRECISION,
    adult_width_meters DOUBLE PRECISION,
    CONSTRAINT trees_reproductive_role_check CHECK (
        reproductive_role IS NULL
        OR reproductive_role IN ('female', 'male', 'self_fertile', 'parthenocarpic')
    ),
    CHECK (harvest_start_day IS NULL OR harvest_start_day BETWEEN 1 AND 366),
    CHECK (harvest_end_day IS NULL OR harvest_end_day BETWEEN 1 AND 366)
);
