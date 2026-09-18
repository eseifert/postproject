CREATE TABLE schema_migrations (
    version INTEGER PRIMARY KEY CHECK (version > 0),
    applied_at_micros INTEGER NOT NULL
);

CREATE TABLE projects (
    singleton INTEGER PRIMARY KEY CHECK (singleton = 1),
    id BLOB NOT NULL UNIQUE CHECK (length(id) = 16),
    schema_version INTEGER NOT NULL CHECK (schema_version > 0),
    created_at_micros INTEGER NOT NULL,
    display_name TEXT
);

CREATE TABLE assets (
    id BLOB PRIMARY KEY CHECK (length(id) = 16),
    created_at_micros INTEGER NOT NULL,
    display_name TEXT,
    import_source TEXT
);

CREATE TABLE representations (
    id BLOB PRIMARY KEY CHECK (length(id) = 16),
    asset_id BLOB NOT NULL REFERENCES assets(id) ON DELETE CASCADE,
    kind INTEGER NOT NULL CHECK (kind BETWEEN 0 AND 3),
    file_size_bytes INTEGER CHECK (file_size_bytes >= 0),
    modified_at_micros INTEGER
);

CREATE INDEX representations_by_asset ON representations(asset_id);

CREATE TABLE fingerprints (
    representation_id BLOB PRIMARY KEY
        REFERENCES representations(id) ON DELETE CASCADE,
    algorithm TEXT NOT NULL CHECK (length(algorithm) BETWEEN 1 AND 64),
    algorithm_version INTEGER NOT NULL CHECK (algorithm_version BETWEEN 0 AND 65535),
    value BLOB NOT NULL CHECK (length(value) > 0)
);

CREATE TABLE locations (
    id BLOB PRIMARY KEY CHECK (length(id) = 16),
    representation_id BLOB NOT NULL
        REFERENCES representations(id) ON DELETE CASCADE,
    uri TEXT NOT NULL CHECK (length(uri) > 0),
    last_seen_micros INTEGER,
    availability INTEGER NOT NULL CHECK (availability BETWEEN 0 AND 2),
    UNIQUE (representation_id, uri)
);

CREATE INDEX locations_by_representation ON locations(representation_id);

CREATE TABLE media_roots (
    id BLOB PRIMARY KEY CHECK (length(id) = 16),
    uri TEXT NOT NULL UNIQUE CHECK (length(uri) > 0),
    label TEXT,
    priority INTEGER NOT NULL,
    enabled INTEGER NOT NULL CHECK (enabled IN (0, 1))
);

CREATE INDEX enabled_media_roots_by_priority
    ON media_roots(priority, id)
    WHERE enabled = 1;

