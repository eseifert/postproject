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

-- structure_kind: 0 single resource, 1 image sequence, 2 ordered parts,
-- 3 package.
CREATE TABLE representations (
    id BLOB PRIMARY KEY CHECK (length(id) = 16),
    asset_id BLOB NOT NULL REFERENCES assets(id) ON DELETE CASCADE,
    kind INTEGER NOT NULL CHECK (kind BETWEEN 0 AND 3),
    structure_kind INTEGER NOT NULL CHECK (structure_kind BETWEEN 0 AND 3)
);

CREATE INDEX representations_by_asset ON representations(asset_id);

CREATE TABLE resources (
    id BLOB PRIMARY KEY CHECK (length(id) = 16),
    file_size_bytes INTEGER CHECK (file_size_bytes >= 0),
    modified_at_micros INTEGER,
    CHECK (file_size_bytes IS NOT NULL OR modified_at_micros IS NULL)
);

CREATE TABLE representation_resources (
    representation_id BLOB NOT NULL
        REFERENCES representations(id) ON DELETE CASCADE,
    resource_id BLOB NOT NULL REFERENCES resources(id) ON DELETE RESTRICT,
    position INTEGER NOT NULL CHECK (position >= 0),
    role TEXT COLLATE BINARY
        CHECK (
            role IS NULL OR
            length(CAST(role AS BLOB)) BETWEEN 1 AND 128
        ),
    required INTEGER NOT NULL CHECK (required IN (0, 1)),
    PRIMARY KEY (representation_id, resource_id),
    UNIQUE (representation_id, position)
);

CREATE INDEX representation_resources_by_resource
    ON representation_resources(resource_id, representation_id);

CREATE TABLE image_sequences (
    representation_id BLOB PRIMARY KEY
        REFERENCES representations(id) ON DELETE CASCADE,
    resource_id BLOB NOT NULL UNIQUE REFERENCES resources(id) ON DELETE RESTRICT,
    prefix TEXT NOT NULL COLLATE BINARY,
    suffix TEXT NOT NULL COLLATE BINARY,
    padding INTEGER NOT NULL CHECK (padding BETWEEN 0 AND 32),
    start_frame INTEGER NOT NULL,
    end_frame INTEGER NOT NULL,
    frame_step INTEGER NOT NULL CHECK (frame_step BETWEEN 1 AND 4294967295),
    rate_numerator INTEGER NOT NULL CHECK (rate_numerator BETWEEN 1 AND 4294967295),
    rate_denominator INTEGER NOT NULL CHECK (rate_denominator BETWEEN 1 AND 4294967295),
    CHECK (end_frame >= start_frame),
    CHECK (length(CAST(prefix AS BLOB)) + length(CAST(suffix AS BLOB)) <= 1024)
);

CREATE TABLE image_sequence_missing_frames (
    representation_id BLOB NOT NULL
        REFERENCES image_sequences(representation_id) ON DELETE CASCADE,
    frame INTEGER NOT NULL,
    PRIMARY KEY (representation_id, frame)
);

CREATE TABLE resource_fingerprints (
    resource_id BLOB NOT NULL REFERENCES resources(id) ON DELETE CASCADE,
    algorithm TEXT NOT NULL CHECK (length(algorithm) BETWEEN 1 AND 64),
    algorithm_version INTEGER NOT NULL CHECK (algorithm_version BETWEEN 0 AND 65535),
    value BLOB NOT NULL CHECK (length(value) > 0),
    PRIMARY KEY (resource_id, algorithm, algorithm_version)
);

CREATE TABLE representation_fingerprints (
    representation_id BLOB NOT NULL
        REFERENCES representations(id) ON DELETE CASCADE,
    algorithm TEXT NOT NULL CHECK (length(algorithm) BETWEEN 1 AND 64),
    algorithm_version INTEGER NOT NULL CHECK (algorithm_version BETWEEN 0 AND 65535),
    value BLOB NOT NULL CHECK (length(value) > 0),
    PRIMARY KEY (representation_id, algorithm, algorithm_version)
);

CREATE TABLE locators (
    id BLOB PRIMARY KEY CHECK (length(id) = 16),
    resource_id BLOB NOT NULL REFERENCES resources(id) ON DELETE CASCADE,
    uri TEXT NOT NULL CHECK (length(uri) > 0),
    last_seen_micros INTEGER,
    availability INTEGER NOT NULL CHECK (availability BETWEEN 0 AND 2),
    UNIQUE (resource_id, uri)
);

CREATE INDEX locators_by_resource ON locators(resource_id, id);

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

-- target_kind: 1 asset, 2 representation, 3 resource.
CREATE TABLE external_identifiers (
    id INTEGER PRIMARY KEY,
    target_kind INTEGER NOT NULL CHECK (target_kind BETWEEN 1 AND 3),
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
    ON external_identifiers(target_kind, target_id, id);

CREATE INDEX external_identifiers_by_scheme_value
    ON external_identifiers(scheme, value, target_kind, target_id);

-- target_kind: 0 project, 1 asset, 2 representation, 3 resource, 4 activity.
CREATE TABLE metadata_assertions (
    id INTEGER PRIMARY KEY,
    target_kind INTEGER NOT NULL CHECK (target_kind BETWEEN 0 AND 4),
    target_id BLOB NOT NULL CHECK (length(target_id) = 16),
    vocabulary TEXT NOT NULL COLLATE BINARY
        CHECK (length(CAST(vocabulary AS BLOB)) BETWEEN 1 AND 512),
    property TEXT NOT NULL COLLATE BINARY
        CHECK (length(CAST(property AS BLOB)) BETWEEN 1 AND 255),
    position INTEGER NOT NULL CHECK (position >= 0),
    encoded_value BLOB NOT NULL
        CHECK (length(encoded_value) BETWEEN 6 AND 16777216),
    UNIQUE (target_kind, target_id, vocabulary, property, position)
);

CREATE INDEX metadata_assertions_by_target
    ON metadata_assertions(
        target_kind,
        target_id,
        vocabulary,
        property,
        position,
        id
    );

CREATE INDEX metadata_assertions_by_property
    ON metadata_assertions(
        vocabulary,
        property,
        target_kind,
        target_id,
        position,
        id
    );

CREATE TRIGGER delete_asset_attachments
AFTER DELETE ON assets
BEGIN
    DELETE FROM external_identifiers WHERE target_kind = 1 AND target_id = OLD.id;
    DELETE FROM metadata_assertions WHERE target_kind = 1 AND target_id = OLD.id;
END;

CREATE TRIGGER delete_representation_attachments
AFTER DELETE ON representations
BEGIN
    DELETE FROM external_identifiers WHERE target_kind = 2 AND target_id = OLD.id;
    DELETE FROM metadata_assertions WHERE target_kind = 2 AND target_id = OLD.id;
END;

CREATE TRIGGER delete_resource_attachments
AFTER DELETE ON resources
BEGIN
    DELETE FROM external_identifiers WHERE target_kind = 3 AND target_id = OLD.id;
    DELETE FROM metadata_assertions WHERE target_kind = 3 AND target_id = OLD.id;
END;
