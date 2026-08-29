ALTER TABLE trees
    ADD COLUMN IF NOT EXISTS is_in_danger BOOLEAN NOT NULL DEFAULT FALSE;

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1
        FROM pg_constraint
        WHERE conrelid = 'trees'::regclass
          AND conname = 'trees_danger_requires_alive_check'
    ) THEN
        ALTER TABLE trees
            ADD CONSTRAINT trees_danger_requires_alive_check CHECK (
                NOT is_in_danger OR is_alive
            );
    END IF;
END $$;
