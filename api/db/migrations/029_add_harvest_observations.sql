CREATE TABLE harvest_observations (
    id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    orchard_id BIGINT NOT NULL REFERENCES orchards(id) ON DELETE CASCADE,
    harvest_run_id BIGINT NOT NULL,
    tree_id BIGINT NOT NULL,
    plant_identity_id BIGINT REFERENCES plant_identities(id) ON DELETE RESTRICT,
    cultivar_id BIGINT REFERENCES plant_cultivars(id) ON DELETE RESTRICT,
    recorded_on DATE NOT NULL,
    kind TEXT NOT NULL CHECK (kind IN ('too_soon', 'too_late')),
    harvested_parts TEXT[] NOT NULL CHECK (cardinality(harvested_parts) > 0 AND harvested_parts <@ ARRAY['cone','flower','fruit','leaf','nut','pod','seed']::TEXT[]),
    window_start DATE NOT NULL,
    window_end DATE NOT NULL CHECK (window_end >= window_start),
    reviewed BOOLEAN NOT NULL DEFAULT FALSE,
    CHECK ((plant_identity_id IS NOT NULL)::integer + (cultivar_id IS NOT NULL)::integer = 1),
    FOREIGN KEY (harvest_run_id, orchard_id) REFERENCES harvest_runs(id, orchard_id) ON DELETE CASCADE,
    FOREIGN KEY (tree_id, orchard_id) REFERENCES trees(id, orchard_id) ON DELETE RESTRICT
);
CREATE INDEX harvest_observations_scope_idx ON harvest_observations (orchard_id, plant_identity_id, cultivar_id, id);

ALTER TABLE harvest_run_trees DROP CONSTRAINT harvest_run_trees_outcome_check;
ALTER TABLE harvest_run_trees ADD CONSTRAINT harvest_run_trees_outcome_check
    CHECK (outcome IN ('harvested', 'deferred', 'done_for_window', 'too_soon', 'too_late'));
ALTER TABLE harvest_run_trees DROP CONSTRAINT harvest_run_trees_resolution_check;
ALTER TABLE harvest_run_trees ADD CONSTRAINT harvest_run_trees_resolution_check CHECK (
    (outcome IS NULL AND resolved_on IS NULL AND retry_on IS NULL)
    OR (outcome IS NOT NULL AND resolved_on IS NOT NULL AND (
        (outcome IN ('harvested', 'done_for_window', 'too_late') AND retry_on IS NULL)
        OR (outcome IN ('deferred', 'too_soon') AND retry_on > resolved_on AND retry_on IS NOT NULL)
    ))
);
