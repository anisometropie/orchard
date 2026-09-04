DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM trees) OR EXISTS (SELECT 1 FROM plant_identities) THEN
        RAISE EXCEPTION 'cannot remove the initial schema while it contains orchard data';
    END IF;
END $$;

DROP TABLE trees;
DROP TABLE plant_identities;
