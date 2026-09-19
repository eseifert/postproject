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

-- target_kind 1 = Asset, 2 = Representation. A polymorphic target cannot use
-- one SQLite foreign key, so insertion triggers enforce existence and deletion
-- triggers provide cascade behavior.
CREATE TABLE external_identifiers (
    id INTEGER PRIMARY KEY,
    target_kind INTEGER NOT NULL CHECK (target_kind IN (1, 2)),
    target_id BLOB NOT NULL CHECK (length(target_id) = 16),
    scheme TEXT NOT NULL COLLATE BINARY
        CHECK (length(CAST(scheme AS BLOB)) BETWEEN 1 AND 255),
    value TEXT NOT NULL COLLATE BINARY
        CHECK (length(CAST(value AS BLOB)) BETWEEN 1 AND 4096),
    qualifier TEXT COLLATE BINARY
        CHECK (
            qualifier IS NULL OR
            length(CAST(qualifier AS BLOB)) BETWEEN 1 AND 1024
        )
);

CREATE UNIQUE INDEX external_identifiers_unique_attachment
    ON external_identifiers (
        target_kind,
        target_id,
        scheme,
        value,
        ifnull(qualifier, '')
    );

CREATE INDEX external_identifiers_by_target
    ON external_identifiers (target_kind, target_id, id);

CREATE INDEX external_identifiers_by_scheme_value
    ON external_identifiers (scheme, value, target_kind, target_id);

CREATE TRIGGER external_identifier_asset_exists
BEFORE INSERT ON external_identifiers
WHEN NEW.target_kind = 1
     AND NOT EXISTS (SELECT 1 FROM assets WHERE id = NEW.target_id)
BEGIN
    SELECT RAISE(ABORT, 'external identifier asset does not exist');
END;

CREATE TRIGGER external_identifier_representation_exists
BEFORE INSERT ON external_identifiers
WHEN NEW.target_kind = 2
     AND NOT EXISTS (
         SELECT 1 FROM representations WHERE id = NEW.target_id
     )
BEGIN
    SELECT RAISE(ABORT, 'external identifier representation does not exist');
END;

CREATE TRIGGER delete_asset_external_identifiers
AFTER DELETE ON assets
BEGIN
    DELETE FROM external_identifiers
    WHERE target_kind = 1 AND target_id = OLD.id;
END;

CREATE TRIGGER delete_representation_external_identifiers
AFTER DELETE ON representations
BEGIN
    DELETE FROM external_identifiers
    WHERE target_kind = 2 AND target_id = OLD.id;
END;
