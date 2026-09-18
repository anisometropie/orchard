ALTER TABLE harvest_runs
    ADD COLUMN harvested_parts TEXT[] NOT NULL DEFAULT ARRAY['fruit']::TEXT[],
    ADD CONSTRAINT harvest_runs_parts_check CHECK (
        cardinality(harvested_parts) > 0
        AND harvested_parts <@ ARRAY[
            'cone', 'flower', 'fruit', 'leaf', 'nut', 'pod', 'seed'
        ]::TEXT[]
    );

ALTER TABLE harvest_run_trees
    ADD COLUMN harvested_parts TEXT[] NOT NULL DEFAULT ARRAY['fruit']::TEXT[],
    ADD CONSTRAINT harvest_run_trees_parts_check CHECK (
        cardinality(harvested_parts) > 0
        AND harvested_parts <@ ARRAY[
            'cone', 'flower', 'fruit', 'leaf', 'nut', 'pod', 'seed'
        ]::TEXT[]
    );
