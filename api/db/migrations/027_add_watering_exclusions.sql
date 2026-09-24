ALTER TABLE trees
    ADD COLUMN is_excluded_from_watering BOOLEAN NOT NULL DEFAULT FALSE;
