CREATE TABLE harvest_runs (
    id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    orchard_id BIGINT NOT NULL REFERENCES orchards(id) ON DELETE CASCADE,
    target_kind TEXT NOT NULL CHECK (target_kind IN ('all', 'species')),
    plant_identity_id BIGINT REFERENCES plant_identities(id) ON DELETE RESTRICT,
    started_on DATE NOT NULL,
    completed_at TIMESTAMPTZ,
    CONSTRAINT harvest_runs_id_orchard_id_key UNIQUE (id, orchard_id),
    CONSTRAINT harvest_runs_target_check CHECK (
        (target_kind = 'all' AND plant_identity_id IS NULL)
        OR (target_kind = 'species' AND plant_identity_id IS NOT NULL)
    )
);

CREATE UNIQUE INDEX harvest_runs_one_active_per_orchard_idx
    ON harvest_runs (orchard_id)
    WHERE completed_at IS NULL;

CREATE TABLE harvest_run_trees (
    harvest_run_id BIGINT NOT NULL,
    orchard_id BIGINT NOT NULL,
    tree_id BIGINT NOT NULL,
    route_rank INTEGER NOT NULL CHECK (route_rank > 0),
    window_start DATE NOT NULL,
    window_end DATE NOT NULL,
    outcome TEXT CHECK (outcome IN ('harvested', 'deferred')),
    resolved_on DATE,
    retry_on DATE,
    PRIMARY KEY (harvest_run_id, tree_id),
    UNIQUE (harvest_run_id, route_rank),
    CONSTRAINT harvest_run_trees_run_orchard_fk
        FOREIGN KEY (harvest_run_id, orchard_id)
        REFERENCES harvest_runs (id, orchard_id)
        ON DELETE CASCADE,
    CONSTRAINT harvest_run_trees_tree_orchard_fk
        FOREIGN KEY (tree_id, orchard_id)
        REFERENCES trees (id, orchard_id)
        ON DELETE RESTRICT,
    CONSTRAINT harvest_run_trees_window_check CHECK (
        window_end >= window_start
    ),
    CONSTRAINT harvest_run_trees_resolution_check CHECK (
        (outcome IS NULL AND resolved_on IS NULL AND retry_on IS NULL)
        OR (
            outcome IS NOT NULL
            AND resolved_on IS NOT NULL
            AND (
                (outcome = 'harvested' AND retry_on IS NULL)
                OR (
                    outcome = 'deferred'
                    AND retry_on IS NOT NULL
                    AND retry_on > resolved_on
                )
            )
        )
    ),
    CONSTRAINT harvest_run_trees_outcome_dates_check CHECK (
        outcome IS NULL
        OR (
            resolved_on BETWEEN window_start AND window_end
            AND (outcome <> 'deferred' OR retry_on <= window_end)
        )
    )
);
