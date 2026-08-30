CREATE TABLE IF NOT EXISTS users (
    id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    username TEXT NOT NULL UNIQUE,
    default_center geometry(Point, 4326) NOT NULL,
    is_default BOOLEAN NOT NULL DEFAULT FALSE
);

CREATE UNIQUE INDEX IF NOT EXISTS users_one_default_idx
    ON users (is_default)
    WHERE is_default;

CREATE TABLE IF NOT EXISTS aerial_overlays (
    id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    image_bytes BYTEA NOT NULL,
    media_type TEXT NOT NULL,
    top_left geometry(Point, 4326) NOT NULL,
    top_right geometry(Point, 4326) NOT NULL,
    bottom_right geometry(Point, 4326) NOT NULL,
    bottom_left geometry(Point, 4326) NOT NULL,
    sort_order INTEGER NOT NULL DEFAULT 0,
    UNIQUE (user_id, name),
    CHECK (octet_length(image_bytes) > 0),
    CHECK (media_type LIKE 'image/%')
);
