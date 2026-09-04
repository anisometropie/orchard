CREATE TABLE orchards (
    id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    owner_user_id BIGINT REFERENCES users(id) ON DELETE SET NULL,
    name TEXT NOT NULL CHECK (btrim(name) <> ''),
    center geometry(Point, 4326) NOT NULL,
    reference_region TEXT NOT NULL CHECK (btrim(reference_region) <> ''),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

ALTER TABLE trees
    ADD COLUMN orchard_id BIGINT REFERENCES orchards(id) ON DELETE CASCADE;

CREATE INDEX trees_orchard_id_idx ON trees (orchard_id);

ALTER TABLE plant_harvest_windows
    ADD COLUMN orchard_id BIGINT REFERENCES orchards(id) ON DELETE CASCADE;

ALTER TABLE plant_harvest_windows
    DROP CONSTRAINT plant_harvest_windows_unique_window,
    ADD CONSTRAINT plant_harvest_windows_unique_window UNIQUE NULLS NOT DISTINCT (
        orchard_id, plant_identity_id, cultivar_id,
        start_month, start_day, end_month, end_day
    );

ALTER TABLE aerial_overlays
    ADD COLUMN orchard_id BIGINT REFERENCES orchards(id) ON DELETE CASCADE;

CREATE INDEX aerial_overlays_orchard_id_idx ON aerial_overlays (orchard_id);

DO $$
DECLARE
    default_user_id BIGINT;
    default_center geometry(Point, 4326);
    default_orchard_id BIGINT;
    inferred_reference_region TEXT;
BEGIN
    SELECT id, users.default_center
    INTO default_user_id, default_center
    FROM users
    WHERE is_default = TRUE
    ORDER BY id
    LIMIT 1;

    IF default_user_id IS NOT NULL THEN
        SELECT COALESCE(
            (
                SELECT reference_region
                FROM plant_harvest_windows
                WHERE reference_region IS NOT NULL
                  AND btrim(reference_region) <> ''
                GROUP BY reference_region
                ORDER BY count(*) DESC, reference_region
                LIMIT 1
            ),
            'Unspecified'
        ) INTO inferred_reference_region;

        INSERT INTO orchards (
            owner_user_id, name, center, reference_region
        ) VALUES (
            default_user_id, 'My orchard', default_center, inferred_reference_region
        )
        RETURNING id INTO default_orchard_id;

        UPDATE trees
        SET orchard_id = default_orchard_id
        WHERE orchard_id IS NULL;

        UPDATE plant_harvest_windows
        SET orchard_id = default_orchard_id
        WHERE orchard_id IS NULL;

        UPDATE aerial_overlays
        SET orchard_id = default_orchard_id
        WHERE orchard_id IS NULL
          AND user_id = default_user_id;
    END IF;
END $$;
