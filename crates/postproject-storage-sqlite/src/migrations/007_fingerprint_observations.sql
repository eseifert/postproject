ALTER TABLE resource_fingerprints
    ADD COLUMN observed_revision_sequence INTEGER CHECK (
        observed_revision_sequence IS NULL OR observed_revision_sequence > 0
    );
ALTER TABLE representation_fingerprints
    ADD COLUMN observed_revision_sequence INTEGER CHECK (
        observed_revision_sequence IS NULL OR observed_revision_sequence > 0
    );

CREATE TABLE resource_fingerprint_history (
    id INTEGER PRIMARY KEY,
    resource_id BLOB NOT NULL REFERENCES resources(id) ON DELETE CASCADE,
    algorithm TEXT NOT NULL COLLATE BINARY,
    algorithm_version INTEGER NOT NULL,
    value BLOB NOT NULL,
    observed_revision_sequence INTEGER,
    superseded_revision_sequence INTEGER NOT NULL,
    CHECK (length(algorithm) BETWEEN 1 AND 64),
    CHECK (algorithm_version BETWEEN 0 AND 65535),
    CHECK (length(value) > 0),
    CHECK (observed_revision_sequence IS NULL OR observed_revision_sequence > 0),
    CHECK (superseded_revision_sequence > 0)
);
CREATE INDEX resource_fingerprint_history_by_owner_domain
    ON resource_fingerprint_history (
        resource_id, algorithm, algorithm_version, superseded_revision_sequence
    );

CREATE TABLE representation_fingerprint_history (
    id INTEGER PRIMARY KEY,
    representation_id BLOB NOT NULL REFERENCES representations(id) ON DELETE CASCADE,
    algorithm TEXT NOT NULL COLLATE BINARY,
    algorithm_version INTEGER NOT NULL,
    value BLOB NOT NULL,
    observed_revision_sequence INTEGER,
    superseded_revision_sequence INTEGER NOT NULL,
    CHECK (length(algorithm) BETWEEN 1 AND 64),
    CHECK (algorithm_version BETWEEN 0 AND 65535),
    CHECK (length(value) > 0),
    CHECK (observed_revision_sequence IS NULL OR observed_revision_sequence > 0),
    CHECK (superseded_revision_sequence > 0)
);
CREATE INDEX representation_fingerprint_history_by_owner_domain
    ON representation_fingerprint_history (
        representation_id, algorithm, algorithm_version, superseded_revision_sequence
    );

CREATE TABLE representation_fingerprint_recomputations (
    representation_id BLOB PRIMARY KEY
        REFERENCES representations(id) ON DELETE CASCADE,
    changed_resource_id BLOB NOT NULL REFERENCES resources(id) ON DELETE CASCADE,
    marked_revision_sequence INTEGER NOT NULL CHECK (marked_revision_sequence > 0)
);

ALTER TABLE activity_inputs
    ADD COLUMN snapshot_revision_sequence INTEGER CHECK (
        snapshot_revision_sequence IS NULL OR snapshot_revision_sequence > 0
    );
ALTER TABLE activity_outputs
    ADD COLUMN snapshot_revision_sequence INTEGER CHECK (
        snapshot_revision_sequence IS NULL OR snapshot_revision_sequence > 0
    );

CREATE TABLE activity_input_fingerprint_snapshots (
    activity_input_id INTEGER NOT NULL REFERENCES activity_inputs(id) ON DELETE CASCADE,
    algorithm TEXT NOT NULL COLLATE BINARY,
    algorithm_version INTEGER NOT NULL,
    value BLOB NOT NULL,
    observed_revision_sequence INTEGER,
    CHECK (length(algorithm) BETWEEN 1 AND 64),
    CHECK (algorithm_version BETWEEN 0 AND 65535),
    CHECK (length(value) > 0),
    CHECK (observed_revision_sequence IS NULL OR observed_revision_sequence > 0),
    PRIMARY KEY (activity_input_id, algorithm, algorithm_version)
);

CREATE TABLE activity_output_fingerprint_snapshots (
    activity_output_id INTEGER NOT NULL REFERENCES activity_outputs(id) ON DELETE CASCADE,
    algorithm TEXT NOT NULL COLLATE BINARY,
    algorithm_version INTEGER NOT NULL,
    value BLOB NOT NULL,
    observed_revision_sequence INTEGER,
    CHECK (length(algorithm) BETWEEN 1 AND 64),
    CHECK (algorithm_version BETWEEN 0 AND 65535),
    CHECK (length(value) > 0),
    CHECK (observed_revision_sequence IS NULL OR observed_revision_sequence > 0),
    PRIMARY KEY (activity_output_id, algorithm, algorithm_version)
);

ALTER TABLE revision_events RENAME TO revision_events_before_fingerprints;

CREATE TABLE revision_events (
    revision_id BLOB NOT NULL REFERENCES revisions(id) ON DELETE CASCADE,
    position INTEGER NOT NULL CHECK (position >= 0),
    kind INTEGER NOT NULL CHECK (kind BETWEEN 1 AND 18),
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
        identifier_scheme IS NULL OR length(CAST(identifier_scheme AS BLOB)) BETWEEN 1 AND 255
    ),
    identifier_value TEXT COLLATE BINARY CHECK (
        identifier_value IS NULL OR length(CAST(identifier_value AS BLOB)) BETWEEN 1 AND 4096
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

INSERT INTO revision_events (
    revision_id, position, kind, target_kind, primary_id, secondary_id,
    structural_position, vocabulary, property, identifier_scheme,
    identifier_value, identifier_qualifier, activity_kind, role
)
SELECT
    revision_id, position, kind, target_kind, primary_id, secondary_id,
    structural_position, vocabulary, property, identifier_scheme,
    identifier_value, identifier_qualifier, activity_kind, role
FROM revision_events_before_fingerprints;

DROP TABLE revision_events_before_fingerprints;

UPDATE productions SET schema_version = 7;
