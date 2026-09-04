DO $$
BEGIN
    IF EXISTS (
        SELECT 1
        FROM trees
        WHERE cultivar_id IS NOT NULL
        GROUP BY plant_identity_id, cultivar_id
        HAVING count(DISTINCT identification_status) > 1
    ) THEN
        RAISE EXCEPTION 'cannot denormalize a cultivar with conflicting identification statuses';
    END IF;
END $$;

ALTER TABLE trees
    DROP CONSTRAINT trees_cultivar_matches_identity_fkey,
    ADD COLUMN harvest_start_day SMALLINT,
    ADD COLUMN harvest_end_day SMALLINT;

ALTER TABLE plant_identities
    DROP CONSTRAINT plant_identities_botanical_taxon_key,
    DROP CONSTRAINT plant_identities_harvest_window_completeness_check,
    DROP CONSTRAINT plant_identities_harvest_start_check,
    DROP CONSTRAINT plant_identities_harvest_end_check,
    ADD COLUMN cultivar TEXT,
    ADD COLUMN trade_name TEXT,
    ADD COLUMN identification_status TEXT,
    ADD COLUMN identity_key TEXT;

CREATE TEMPORARY TABLE reverted_plant_identity_map (
    plant_identity_id BIGINT NOT NULL,
    cultivar_id BIGINT,
    identification_status TEXT NOT NULL,
    reverted_plant_identity_id BIGINT NOT NULL UNIQUE,
    UNIQUE NULLS NOT DISTINCT (
        plant_identity_id,
        cultivar_id,
        identification_status
    )
) ON COMMIT DROP;

DO $$
DECLARE
    identity_variant RECORD;
    reverted_id BIGINT;
BEGIN
    FOR identity_variant IN
        SELECT DISTINCT
            variant.plant_identity_id,
            variant.cultivar_id,
            variant.identification_status
        FROM (
            SELECT
                identity.id AS plant_identity_id,
                NULL::BIGINT AS cultivar_id,
                'confirmed'::TEXT AS identification_status
            FROM plant_identities identity
            WHERE NOT EXISTS (
                SELECT 1
                FROM trees tree
                WHERE tree.plant_identity_id = identity.id
                  AND tree.cultivar_id IS NULL
            )

            UNION

            SELECT
                cultivar.plant_identity_id,
                cultivar.id,
                'confirmed'::TEXT
            FROM plant_cultivars cultivar
            WHERE NOT EXISTS (
                SELECT 1
                FROM trees tree
                WHERE tree.plant_identity_id = cultivar.plant_identity_id
                  AND tree.cultivar_id = cultivar.id
            )

            UNION

            SELECT
                tree.plant_identity_id,
                tree.cultivar_id,
                tree.identification_status
            FROM trees tree
        ) variant
        ORDER BY
            variant.plant_identity_id,
            variant.cultivar_id NULLS FIRST,
            variant.identification_status
    LOOP
        INSERT INTO plant_identities (
            common_name,
            botanical_taxon,
            harvest_start_month,
            harvest_start_day,
            harvest_end_month,
            harvest_end_day,
            cultivar,
            trade_name,
            identification_status,
            identity_key
        )
        SELECT
            identity.common_name,
            identity.botanical_taxon,
            identity.harvest_start_month,
            identity.harvest_start_day,
            identity.harvest_end_month,
            identity.harvest_end_day,
            cultivar.cultivar,
            cultivar.trade_name,
            identity_variant.identification_status,
            format(
                'reverted-v7-%s-%s-%s',
                identity_variant.plant_identity_id,
                coalesce(identity_variant.cultivar_id::TEXT, 'none'),
                identity_variant.identification_status
            )
        FROM plant_identities identity
        LEFT JOIN plant_cultivars cultivar
            ON cultivar.id = identity_variant.cultivar_id
        WHERE identity.id = identity_variant.plant_identity_id
        RETURNING id INTO reverted_id;

        INSERT INTO reverted_plant_identity_map (
            plant_identity_id,
            cultivar_id,
            identification_status,
            reverted_plant_identity_id
        ) VALUES (
            identity_variant.plant_identity_id,
            identity_variant.cultivar_id,
            identity_variant.identification_status,
            reverted_id
        );
    END LOOP;
END $$;

UPDATE trees tree
SET
    plant_identity_id = identity_map.reverted_plant_identity_id,
    harvest_start_day = extract(
        doy FROM make_date(2000, identity.harvest_start_month, identity.harvest_start_day)
    ),
    harvest_end_day = extract(
        doy FROM make_date(2000, identity.harvest_end_month, identity.harvest_end_day)
    )
FROM reverted_plant_identity_map identity_map
JOIN plant_identities identity
    ON identity.id = identity_map.plant_identity_id
WHERE tree.plant_identity_id = identity_map.plant_identity_id
  AND tree.cultivar_id IS NOT DISTINCT FROM identity_map.cultivar_id
  AND tree.identification_status = identity_map.identification_status;

DROP TABLE plant_cultivars;

DELETE FROM plant_identities identity
WHERE identity.id NOT IN (
    SELECT reverted_plant_identity_id
    FROM reverted_plant_identity_map
);

ALTER TABLE plant_identities
    ALTER COLUMN identification_status SET NOT NULL,
    ALTER COLUMN identity_key SET NOT NULL,
    ADD CONSTRAINT plant_identities_identity_key_key UNIQUE (identity_key),
    DROP COLUMN harvest_start_month,
    DROP COLUMN harvest_start_day,
    DROP COLUMN harvest_end_month,
    DROP COLUMN harvest_end_day;

ALTER TABLE trees
    DROP CONSTRAINT trees_identification_status_check,
    DROP COLUMN cultivar_id,
    DROP COLUMN identification_status,
    ADD CHECK (harvest_start_day IS NULL OR harvest_start_day BETWEEN 1 AND 366),
    ADD CHECK (harvest_end_day IS NULL OR harvest_end_day BETWEEN 1 AND 366);
