ALTER TABLE revision_events RENAME TO revision_events_before_lifecycle;

CREATE TABLE revision_events (
    revision_id BLOB NOT NULL REFERENCES revisions(id) ON DELETE CASCADE,
    position INTEGER NOT NULL CHECK (position >= 0),
    kind INTEGER NOT NULL CHECK (kind BETWEEN 1 AND 16),
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
FROM revision_events_before_lifecycle;

DROP TABLE revision_events_before_lifecycle;

UPDATE productions SET schema_version = 5;
