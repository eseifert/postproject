DROP TRIGGER delete_asset_attachments;
DROP TRIGGER delete_representation_attachments;
DROP TRIGGER delete_resource_attachments;
DROP TRIGGER delete_activity_attachments;

ALTER TABLE external_identifiers RENAME TO external_identifiers_before_activity;

-- target_kind: 1 asset, 2 representation, 3 resource, 4 activity.
CREATE TABLE external_identifiers (
    id INTEGER PRIMARY KEY,
    target_kind INTEGER NOT NULL CHECK (target_kind BETWEEN 1 AND 4),
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

INSERT INTO external_identifiers
    (id, target_kind, target_id, scheme, value, qualifier)
SELECT id, target_kind, target_id, scheme, value, qualifier
FROM external_identifiers_before_activity;

DROP TABLE external_identifiers_before_activity;

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

CREATE TRIGGER delete_activity_attachments
AFTER DELETE ON activities
BEGIN
    DELETE FROM external_identifiers WHERE target_kind = 4 AND target_id = OLD.id;
    DELETE FROM metadata_assertions WHERE target_kind = 4 AND target_id = OLD.id;
END;

UPDATE productions SET schema_version = 4;
