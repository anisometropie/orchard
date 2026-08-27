ALTER TABLE trees
    ADD COLUMN IF NOT EXISTS legacy_identification_name TEXT,
    ADD COLUMN IF NOT EXISTS legacy_identification_latin_name TEXT,
    ADD COLUMN IF NOT EXISTS reproductive_role TEXT;

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1
        FROM pg_constraint
        WHERE conrelid = 'trees'::regclass
          AND conname = 'trees_reproductive_role_check'
    ) THEN
        ALTER TABLE trees
            ADD CONSTRAINT trees_reproductive_role_check CHECK (
                reproductive_role IS NULL
                OR reproductive_role IN ('female', 'male', 'self_fertile', 'parthenocarpic')
            );
    END IF;
END $$;
