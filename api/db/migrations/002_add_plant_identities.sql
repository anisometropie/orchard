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

DO $$
BEGIN
    IF EXISTS (
        SELECT 1
        FROM information_schema.columns
        WHERE table_schema = current_schema()
          AND table_name = 'trees'
          AND column_name = 'name'
    ) THEN
        ALTER TABLE trees RENAME COLUMN name TO legacy_name;
    END IF;

    IF EXISTS (
        SELECT 1
        FROM information_schema.columns
        WHERE table_schema = current_schema()
          AND table_name = 'trees'
          AND column_name = 'latin_name'
    ) THEN
        ALTER TABLE trees RENAME COLUMN latin_name TO legacy_latin_name;
    END IF;
END $$;

ALTER TABLE trees
    ADD COLUMN IF NOT EXISTS plant_identity_id BIGINT,
    ADD COLUMN IF NOT EXISTS legacy_name TEXT,
    ADD COLUMN IF NOT EXISTS legacy_latin_name TEXT;

ALTER TABLE trees
    ALTER COLUMN legacy_name DROP NOT NULL;

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1
        FROM pg_constraint
        WHERE conrelid = 'trees'::regclass
          AND conname = 'trees_plant_identity_id_fkey'
    ) THEN
        ALTER TABLE trees
            ADD CONSTRAINT trees_plant_identity_id_fkey
            FOREIGN KEY (plant_identity_id) REFERENCES plant_identities(id);
    END IF;
END $$;
