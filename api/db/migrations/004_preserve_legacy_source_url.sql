ALTER TABLE trees
    ADD COLUMN IF NOT EXISTS legacy_source_url TEXT;
