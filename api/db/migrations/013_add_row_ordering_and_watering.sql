ALTER TABLE trees
    ADD COLUMN row_rank INTEGER,
    ADD CONSTRAINT trees_row_rank_check CHECK (
        row_rank IS NULL OR (
            orchard_id IS NOT NULL
            AND row_name IS NOT NULL
            AND btrim(row_name) <> ''
            AND row_rank > 0
        )
    );

CREATE UNIQUE INDEX trees_orchard_row_rank_key
    ON trees (orchard_id, row_name, row_rank)
    WHERE row_rank IS NOT NULL;

CREATE TABLE watering_runs (
    id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    orchard_id BIGINT NOT NULL REFERENCES orchards(id) ON DELETE CASCADE,
    row_name TEXT NOT NULL CHECK (btrim(row_name) <> ''),
    started_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    completed_at TIMESTAMPTZ,
    CHECK (completed_at IS NULL OR completed_at >= started_at)
);

CREATE UNIQUE INDEX watering_runs_one_active_per_orchard_idx
    ON watering_runs (orchard_id)
    WHERE completed_at IS NULL;

CREATE TABLE watering_run_trees (
    watering_run_id BIGINT NOT NULL REFERENCES watering_runs(id) ON DELETE CASCADE,
    tree_id BIGINT NOT NULL REFERENCES trees(id) ON DELETE RESTRICT,
    row_rank INTEGER NOT NULL CHECK (row_rank > 0),
    watered_at TIMESTAMPTZ,
    PRIMARY KEY (watering_run_id, tree_id),
    UNIQUE (watering_run_id, row_rank)
);
