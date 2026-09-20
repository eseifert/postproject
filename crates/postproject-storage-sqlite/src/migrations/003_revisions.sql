CREATE TABLE revisions (
    id BLOB PRIMARY KEY CHECK (length(id) = 16),
    sequence INTEGER NOT NULL UNIQUE CHECK (sequence > 0),
    transaction_id BLOB NOT NULL UNIQUE CHECK (length(transaction_id) = 16),
    committed_at_micros INTEGER NOT NULL,
    origin_name TEXT COLLATE BINARY
        CHECK (
            origin_name IS NULL OR
            length(CAST(origin_name AS BLOB)) BETWEEN 1 AND 256
        ),
    origin_version TEXT COLLATE BINARY
        CHECK (
            origin_version IS NULL OR
            length(CAST(origin_version AS BLOB)) BETWEEN 1 AND 128
        ),
    origin_uri TEXT COLLATE BINARY
        CHECK (
            origin_uri IS NULL OR
            length(CAST(origin_uri AS BLOB)) BETWEEN 1 AND 4096
        ),
    message TEXT COLLATE BINARY
        CHECK (
            message IS NULL OR
            length(CAST(message AS BLOB)) BETWEEN 1 AND 4096
        ),
    CHECK (
        origin_name IS NOT NULL OR
        (origin_version IS NULL AND origin_uri IS NULL)
    )
);

CREATE INDEX revisions_by_commit_time
    ON revisions(committed_at_micros, sequence);

CREATE TABLE revision_events (
    revision_id BLOB NOT NULL REFERENCES revisions(id) ON DELETE CASCADE,
    position INTEGER NOT NULL CHECK (position >= 0),
    kind INTEGER NOT NULL CHECK (kind BETWEEN 1 AND 13),
    target_kind INTEGER CHECK (target_kind BETWEEN 0 AND 4),
    primary_id BLOB CHECK (primary_id IS NULL OR length(primary_id) = 16),
    secondary_id BLOB CHECK (secondary_id IS NULL OR length(secondary_id) = 16),
    structural_position INTEGER CHECK (structural_position >= 0),
    vocabulary TEXT COLLATE BINARY
        CHECK (
            vocabulary IS NULL OR
            length(CAST(vocabulary AS BLOB)) BETWEEN 1 AND 512
        ),
    property TEXT COLLATE BINARY
        CHECK (
            property IS NULL OR
            length(CAST(property AS BLOB)) BETWEEN 1 AND 255
        ),
    identifier_scheme TEXT COLLATE BINARY
        CHECK (
            identifier_scheme IS NULL OR
            length(CAST(identifier_scheme AS BLOB)) BETWEEN 1 AND 255
        ),
    identifier_value TEXT COLLATE BINARY
        CHECK (
            identifier_value IS NULL OR
            length(CAST(identifier_value AS BLOB)) BETWEEN 1 AND 4096
        ),
    identifier_qualifier TEXT COLLATE BINARY
        CHECK (
            identifier_qualifier IS NULL OR
            length(CAST(identifier_qualifier AS BLOB)) BETWEEN 1 AND 1024
        ),
    activity_kind TEXT COLLATE BINARY
        CHECK (
            activity_kind IS NULL OR
            length(CAST(activity_kind AS BLOB)) BETWEEN 1 AND 128
        ),
    role TEXT COLLATE BINARY
        CHECK (
            role IS NULL OR
            length(CAST(role AS BLOB)) BETWEEN 1 AND 128
        ),
    PRIMARY KEY (revision_id, position)
);

UPDATE projects SET schema_version = 3;
