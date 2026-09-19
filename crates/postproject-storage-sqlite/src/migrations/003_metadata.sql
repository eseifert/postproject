CREATE TABLE metadata_assertions (
    id INTEGER PRIMARY KEY,
    target_kind INTEGER NOT NULL CHECK (target_kind BETWEEN 0 AND 3),
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
    ON metadata_assertions (
        target_kind,
        target_id,
        vocabulary,
        property,
        position,
        id
    );

CREATE INDEX metadata_assertions_by_property
    ON metadata_assertions (
        vocabulary,
        property,
        target_kind,
        target_id,
        position,
        id
    );

CREATE TRIGGER metadata_project_exists
BEFORE INSERT ON metadata_assertions
WHEN NEW.target_kind = 0
     AND NOT EXISTS (SELECT 1 FROM projects WHERE id = NEW.target_id)
BEGIN
    SELECT RAISE(ABORT, 'metadata project does not exist');
END;

CREATE TRIGGER metadata_asset_exists
BEFORE INSERT ON metadata_assertions
WHEN NEW.target_kind = 1
     AND NOT EXISTS (SELECT 1 FROM assets WHERE id = NEW.target_id)
BEGIN
    SELECT RAISE(ABORT, 'metadata asset does not exist');
END;

CREATE TRIGGER metadata_representation_exists
BEFORE INSERT ON metadata_assertions
WHEN NEW.target_kind = 2
     AND NOT EXISTS (
         SELECT 1 FROM representations WHERE id = NEW.target_id
     )
BEGIN
    SELECT RAISE(ABORT, 'metadata representation does not exist');
END;

CREATE TRIGGER delete_asset_metadata
AFTER DELETE ON assets
BEGIN
    DELETE FROM metadata_assertions
    WHERE target_kind = 1 AND target_id = OLD.id;
END;

CREATE TRIGGER delete_representation_metadata
AFTER DELETE ON representations
BEGIN
    DELETE FROM metadata_assertions
    WHERE target_kind = 2 AND target_id = OLD.id;
END;

UPDATE projects SET schema_version = 3;
