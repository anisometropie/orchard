BEGIN;

DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM trees WHERE plant_identity_id IS NULL) THEN
        RAISE EXCEPTION 'cannot normalize trees without a plant identity';
    END IF;

    IF EXISTS (
        SELECT 1
        FROM plant_identities
        WHERE cultivar IS NULL AND trade_name IS NOT NULL
    ) THEN
        RAISE EXCEPTION 'cannot normalize a trade name without a cultivar';
    END IF;

    IF EXISTS (
        SELECT 1
        FROM plant_identities
        WHERE cultivar IS NOT NULL
        GROUP BY botanical_taxon, cultivar
        HAVING count(*) > 1
    ) THEN
        RAISE EXCEPTION 'cannot normalize duplicate cultivars for one botanical taxon';
    END IF;

    IF EXISTS (
        SELECT 1
        FROM trees
        WHERE (harvest_start_day IS NULL) <> (harvest_end_day IS NULL)
    ) THEN
        RAISE EXCEPTION 'cannot normalize partial tree harvest windows';
    END IF;

    IF EXISTS (
        SELECT 1
        FROM trees t
        JOIN plant_identities p ON p.id = t.plant_identity_id
        WHERE t.harvest_start_day IS NOT NULL
        GROUP BY p.botanical_taxon
        HAVING count(DISTINCT ROW(t.harvest_start_day, t.harvest_end_day)) > 1
    ) THEN
        RAISE EXCEPTION 'cannot normalize conflicting harvest windows for one botanical taxon';
    END IF;
END $$;

CREATE TEMPORARY TABLE plant_identity_merge_map ON COMMIT DROP AS
SELECT
    id AS old_plant_identity_id,
    min(id) OVER (PARTITION BY botanical_taxon) AS plant_identity_id
FROM plant_identities;

CREATE TEMPORARY TABLE plant_identity_common_names ON COMMIT DROP AS
SELECT DISTINCT ON (p.botanical_taxon)
    p.botanical_taxon,
    p.common_name
FROM plant_identities p
LEFT JOIN trees t ON t.plant_identity_id = p.id
GROUP BY p.botanical_taxon, p.common_name
ORDER BY
    p.botanical_taxon,
    count(t.id) DESC,
    bool_or(p.cultivar IS NULL AND p.identification_status = 'confirmed') DESC,
    length(p.common_name),
    p.common_name;

CREATE TABLE plant_cultivars (
    id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    plant_identity_id BIGINT NOT NULL REFERENCES plant_identities(id) ON DELETE CASCADE,
    cultivar TEXT NOT NULL,
    trade_name TEXT,
    UNIQUE (plant_identity_id, cultivar),
    UNIQUE (id, plant_identity_id),
    CHECK (btrim(cultivar) <> '')
);

ALTER TABLE plant_identities
    ADD COLUMN harvest_start_month SMALLINT,
    ADD COLUMN harvest_start_day SMALLINT,
    ADD COLUMN harvest_end_month SMALLINT,
    ADD COLUMN harvest_end_day SMALLINT;

ALTER TABLE trees
    ADD COLUMN cultivar_id BIGINT,
    ADD COLUMN identification_status TEXT NOT NULL DEFAULT 'confirmed';

INSERT INTO plant_cultivars (plant_identity_id, cultivar, trade_name)
SELECT
    identity_map.plant_identity_id,
    old_identity.cultivar,
    old_identity.trade_name
FROM plant_identities old_identity
JOIN plant_identity_merge_map identity_map
    ON identity_map.old_plant_identity_id = old_identity.id
WHERE old_identity.cultivar IS NOT NULL;

UPDATE trees tree
SET
    plant_identity_id = identity_map.plant_identity_id,
    cultivar_id = cultivar.id,
    identification_status = old_identity.identification_status
FROM plant_identities old_identity
JOIN plant_identity_merge_map identity_map
    ON identity_map.old_plant_identity_id = old_identity.id
LEFT JOIN plant_cultivars cultivar
    ON cultivar.plant_identity_id = identity_map.plant_identity_id
   AND cultivar.cultivar = old_identity.cultivar
WHERE tree.plant_identity_id = old_identity.id;

WITH harvest_windows AS (
    SELECT DISTINCT ON (identity_map.plant_identity_id)
        identity_map.plant_identity_id,
        tree.harvest_start_day,
        tree.harvest_end_day
    FROM trees tree
    JOIN plant_identity_merge_map identity_map
        ON identity_map.plant_identity_id = tree.plant_identity_id
    WHERE tree.harvest_start_day IS NOT NULL
    ORDER BY identity_map.plant_identity_id
)
UPDATE plant_identities identity
SET
    harvest_start_month = extract(
        month FROM DATE '2000-01-01' + (harvest_windows.harvest_start_day - 1)
    ),
    harvest_start_day = extract(
        day FROM DATE '2000-01-01' + (harvest_windows.harvest_start_day - 1)
    ),
    harvest_end_month = extract(
        month FROM DATE '2000-01-01' + (harvest_windows.harvest_end_day - 1)
    ),
    harvest_end_day = extract(
        day FROM DATE '2000-01-01' + (harvest_windows.harvest_end_day - 1)
    )
FROM harvest_windows
WHERE identity.id = harvest_windows.plant_identity_id;

UPDATE plant_identities identity
SET common_name = common_names.common_name
FROM plant_identity_common_names common_names
WHERE identity.botanical_taxon = common_names.botanical_taxon
  AND identity.id IN (
      SELECT plant_identity_id FROM plant_identity_merge_map
  );

DELETE FROM plant_identities identity
WHERE identity.id NOT IN (
    SELECT plant_identity_id FROM plant_identity_merge_map
);

ALTER TABLE plant_identities
    DROP COLUMN cultivar,
    DROP COLUMN trade_name,
    DROP COLUMN identification_status,
    DROP COLUMN identity_key,
    ADD CONSTRAINT plant_identities_botanical_taxon_key UNIQUE (botanical_taxon),
    ADD CONSTRAINT plant_identities_harvest_window_completeness_check CHECK (
        (
            harvest_start_month IS NULL
            AND harvest_start_day IS NULL
            AND harvest_end_month IS NULL
            AND harvest_end_day IS NULL
        )
        OR
        (
            harvest_start_month IS NOT NULL
            AND harvest_start_day IS NOT NULL
            AND harvest_end_month IS NOT NULL
            AND harvest_end_day IS NOT NULL
        )
    ),
    ADD CONSTRAINT plant_identities_harvest_start_check CHECK (
        harvest_start_month IS NULL
        OR (
            harvest_start_month BETWEEN 1 AND 12
            AND harvest_start_day BETWEEN 1 AND CASE harvest_start_month
                WHEN 2 THEN 29
                WHEN 4 THEN 30
                WHEN 6 THEN 30
                WHEN 9 THEN 30
                WHEN 11 THEN 30
                ELSE 31
            END
        )
    ),
    ADD CONSTRAINT plant_identities_harvest_end_check CHECK (
        harvest_end_month IS NULL
        OR (
            harvest_end_month BETWEEN 1 AND 12
            AND harvest_end_day BETWEEN 1 AND CASE harvest_end_month
                WHEN 2 THEN 29
                WHEN 4 THEN 30
                WHEN 6 THEN 30
                WHEN 9 THEN 30
                WHEN 11 THEN 30
                ELSE 31
            END
        )
    );

ALTER TABLE trees
    DROP COLUMN harvest_start_day,
    DROP COLUMN harvest_end_day,
    ADD CONSTRAINT trees_identification_status_check CHECK (
        identification_status IN ('confirmed', 'uncertain')
    ),
    ADD CONSTRAINT trees_cultivar_matches_identity_fkey
        FOREIGN KEY (cultivar_id, plant_identity_id)
        REFERENCES plant_cultivars(id, plant_identity_id);

COMMIT;
