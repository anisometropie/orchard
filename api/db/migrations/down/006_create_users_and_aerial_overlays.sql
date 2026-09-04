DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM aerial_overlays) OR EXISTS (SELECT 1 FROM users) THEN
        RAISE EXCEPTION 'cannot remove users and aerial overlays while they contain data';
    END IF;
END $$;

DROP TABLE aerial_overlays;
DROP TABLE users;
