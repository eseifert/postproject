CREATE TABLE dependency_sets (
    source_representation_id BLOB PRIMARY KEY
        REFERENCES representations(id) ON DELETE CASCADE,
    recorded_revision_sequence INTEGER NOT NULL
        CHECK (recorded_revision_sequence > 0),
    needs_extraction INTEGER NOT NULL CHECK (needs_extraction IN (0, 1))
);

CREATE TABLE dependencies (
    source_representation_id BLOB NOT NULL
        REFERENCES dependency_sets(source_representation_id) ON DELETE CASCADE,
    position INTEGER NOT NULL CHECK (position >= 0),
    source_resource_id BLOB REFERENCES resources(id) ON DELETE RESTRICT,
    kind TEXT NOT NULL COLLATE BINARY
        CHECK (length(CAST(kind AS BLOB)) BETWEEN 1 AND 128),
    target_kind INTEGER NOT NULL CHECK (target_kind IN (1, 2)),
    target_id BLOB NOT NULL CHECK (length(target_id) = 16),
    resolved_representation_id BLOB
        REFERENCES representations(id) ON DELETE RESTRICT,
    required INTEGER NOT NULL CHECK (required IN (0, 1)),
    authored_reference TEXT NOT NULL COLLATE BINARY CHECK (
        length(CAST(authored_reference AS BLOB)) <= 4096 AND
        instr(authored_reference, char(0)) = 0
    ),
    CHECK (
        (target_kind = 1) OR
        (target_kind = 2 AND resolved_representation_id IS NULL)
    ),
    PRIMARY KEY (source_representation_id, position)
);

CREATE INDEX dependencies_by_target
    ON dependencies(target_kind, target_id, source_representation_id, position);
CREATE INDEX dependencies_by_resolved_representation
    ON dependencies(resolved_representation_id, source_representation_id, position)
    WHERE resolved_representation_id IS NOT NULL;

CREATE TRIGGER validate_dependency_insert
BEFORE INSERT ON dependencies
BEGIN
    SELECT CASE
        WHEN NEW.source_resource_id IS NOT NULL AND NOT EXISTS (
            SELECT 1 FROM representation_resources
            WHERE representation_id = NEW.source_representation_id
              AND resource_id = NEW.source_resource_id
        ) THEN RAISE(ABORT, 'dependency source resource is not a member')
        WHEN NEW.target_kind = 1 AND NOT EXISTS (
            SELECT 1 FROM assets WHERE id = NEW.target_id
        ) THEN RAISE(ABORT, 'dependency target asset does not exist')
        WHEN NEW.target_kind = 2 AND NOT EXISTS (
            SELECT 1 FROM representations WHERE id = NEW.target_id
        ) THEN RAISE(ABORT, 'dependency target representation does not exist')
        WHEN NEW.resolved_representation_id IS NOT NULL AND NOT EXISTS (
            SELECT 1 FROM representations
            WHERE id = NEW.resolved_representation_id
              AND asset_id = NEW.target_id
        ) THEN RAISE(ABORT, 'resolved dependency representation does not belong to target asset')
    END;
END;

CREATE TABLE activity_input_dependency_snapshots (
    activity_input_id INTEGER PRIMARY KEY
        REFERENCES activity_inputs(id) ON DELETE CASCADE
);

CREATE TABLE activity_input_dependency_paths (
    id INTEGER PRIMARY KEY,
    activity_input_id INTEGER NOT NULL
        REFERENCES activity_input_dependency_snapshots(activity_input_id)
        ON DELETE CASCADE,
    position INTEGER NOT NULL CHECK (position >= 0),
    status INTEGER NOT NULL CHECK (status BETWEEN 0 AND 4),
    subject_representation_id BLOB NOT NULL
        REFERENCES representations(id) ON DELETE RESTRICT,
    UNIQUE (activity_input_id, position)
);

CREATE TABLE activity_input_dependency_path_edges (
    path_id INTEGER NOT NULL
        REFERENCES activity_input_dependency_paths(id) ON DELETE CASCADE,
    position INTEGER NOT NULL CHECK (position >= 0),
    source_representation_id BLOB NOT NULL
        REFERENCES representations(id) ON DELETE RESTRICT,
    dependency_position INTEGER NOT NULL CHECK (dependency_position >= 0),
    source_resource_id BLOB REFERENCES resources(id) ON DELETE RESTRICT,
    kind TEXT NOT NULL COLLATE BINARY
        CHECK (length(CAST(kind AS BLOB)) BETWEEN 1 AND 128),
    target_kind INTEGER NOT NULL CHECK (target_kind IN (1, 2)),
    target_id BLOB NOT NULL CHECK (length(target_id) = 16),
    resolved_representation_id BLOB
        REFERENCES representations(id) ON DELETE RESTRICT,
    authored_reference TEXT NOT NULL COLLATE BINARY CHECK (
        length(CAST(authored_reference AS BLOB)) <= 4096 AND
        instr(authored_reference, char(0)) = 0
    ),
    CHECK (
        (target_kind = 1) OR
        (target_kind = 2 AND resolved_representation_id IS NULL)
    ),
    PRIMARY KEY (path_id, position)
);

CREATE TABLE activity_input_dependency_fingerprint_snapshots (
    path_id INTEGER NOT NULL
        REFERENCES activity_input_dependency_paths(id) ON DELETE CASCADE,
    algorithm TEXT NOT NULL COLLATE BINARY,
    algorithm_version INTEGER NOT NULL,
    value BLOB NOT NULL,
    observed_revision_sequence INTEGER,
    CHECK (length(algorithm) BETWEEN 1 AND 64),
    CHECK (algorithm_version BETWEEN 0 AND 65535),
    CHECK (length(value) > 0),
    CHECK (observed_revision_sequence IS NULL OR observed_revision_sequence > 0),
    PRIMARY KEY (path_id, algorithm, algorithm_version)
);

ALTER TABLE revision_events RENAME TO revision_events_before_dependencies;

CREATE TABLE revision_events (
    revision_id BLOB NOT NULL REFERENCES revisions(id) ON DELETE CASCADE,
    position INTEGER NOT NULL CHECK (position >= 0),
    kind INTEGER NOT NULL CHECK (kind BETWEEN 1 AND 19),
    target_kind INTEGER CHECK (target_kind BETWEEN 0 AND 4),
    primary_id BLOB CHECK (primary_id IS NULL OR length(primary_id) = 16),
    secondary_id BLOB CHECK (secondary_id IS NULL OR length(secondary_id) = 16),
    structural_position INTEGER CHECK (structural_position >= 0),
    vocabulary TEXT COLLATE BINARY CHECK (
        vocabulary IS NULL OR length(CAST(vocabulary AS BLOB)) BETWEEN 1 AND 512
    ),
    property TEXT COLLATE BINARY CHECK (
        property IS NULL OR length(CAST(property AS BLOB)) BETWEEN 1 AND 255
    ),
    identifier_scheme TEXT COLLATE BINARY CHECK (
        identifier_scheme IS NULL OR
        length(CAST(identifier_scheme AS BLOB)) BETWEEN 1 AND 255
    ),
    identifier_value TEXT COLLATE BINARY CHECK (
        identifier_value IS NULL OR
        length(CAST(identifier_value AS BLOB)) BETWEEN 1 AND 4096
    ),
    identifier_qualifier TEXT COLLATE BINARY CHECK (
        identifier_qualifier IS NULL OR
        length(CAST(identifier_qualifier AS BLOB)) BETWEEN 1 AND 1024
    ),
    activity_kind TEXT COLLATE BINARY CHECK (
        activity_kind IS NULL OR length(CAST(activity_kind AS BLOB)) BETWEEN 1 AND 128
    ),
    role TEXT COLLATE BINARY CHECK (
        role IS NULL OR length(CAST(role AS BLOB)) BETWEEN 1 AND 128
    ),
    fingerprint_algorithm TEXT COLLATE BINARY CHECK (
        fingerprint_algorithm IS NULL OR
        length(CAST(fingerprint_algorithm AS BLOB)) BETWEEN 1 AND 64
    ),
    fingerprint_version INTEGER CHECK (
        fingerprint_version IS NULL OR fingerprint_version BETWEEN 0 AND 65535
    ),
    PRIMARY KEY (revision_id, position)
);

INSERT INTO revision_events SELECT * FROM revision_events_before_dependencies;
DROP TABLE revision_events_before_dependencies;

UPDATE productions SET schema_version = 8;
