CREATE UNIQUE INDEX trees_id_orchard_id_key
    ON trees (id, orchard_id);

CREATE TABLE tree_photos (
    id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    orchard_id BIGINT NOT NULL,
    tree_id BIGINT NOT NULL,
    image_webp BYTEA NOT NULL,
    thumbnail_webp BYTEA NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT tree_photos_tree_orchard_fk
        FOREIGN KEY (tree_id, orchard_id)
        REFERENCES trees (id, orchard_id)
        ON DELETE CASCADE,
    CONSTRAINT tree_photos_image_size_check CHECK (
        octet_length(image_webp) BETWEEN 12 AND 8388608
    ),
    CONSTRAINT tree_photos_thumbnail_size_check CHECK (
        octet_length(thumbnail_webp) BETWEEN 12 AND 524288
    )
);

CREATE INDEX tree_photos_latest_tree_idx
    ON tree_photos (orchard_id, tree_id, id DESC);
