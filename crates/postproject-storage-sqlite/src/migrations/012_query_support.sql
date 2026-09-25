-- Derived query-support tables. Triggers keep them consistent with the
-- authoritative rows on every write path, so no caller maintains them.

-- Required memberships whose resource has no durable locator. Keyed by
-- representation so unresolved-media pages read only unresolved rows.
CREATE TABLE unresolved_memberships (
    representation_id BLOB NOT NULL
        REFERENCES representations(id) ON DELETE CASCADE,
    resource_id BLOB NOT NULL REFERENCES resources(id) ON DELETE CASCADE,
    PRIMARY KEY (representation_id, resource_id)
) WITHOUT ROWID;
CREATE INDEX unresolved_memberships_by_resource
    ON unresolved_memberships(resource_id);

INSERT INTO unresolved_memberships (representation_id, resource_id)
SELECT rr.representation_id, rr.resource_id
FROM representation_resources rr
WHERE rr.required = 1
  AND NOT EXISTS (SELECT 1 FROM locators l WHERE l.resource_id = rr.resource_id);

CREATE TRIGGER unresolved_memberships_after_membership_insert
AFTER INSERT ON representation_resources
WHEN NEW.required = 1
 AND NOT EXISTS (SELECT 1 FROM locators WHERE resource_id = NEW.resource_id)
BEGIN
    INSERT OR IGNORE INTO unresolved_memberships (representation_id, resource_id)
    VALUES (NEW.representation_id, NEW.resource_id);
END;

CREATE TRIGGER unresolved_memberships_after_membership_update
AFTER UPDATE OF representation_id, resource_id, required ON representation_resources
BEGIN
    DELETE FROM unresolved_memberships
    WHERE representation_id = OLD.representation_id AND resource_id = OLD.resource_id;
    INSERT OR IGNORE INTO unresolved_memberships (representation_id, resource_id)
    SELECT NEW.representation_id, NEW.resource_id
    WHERE NEW.required = 1
      AND NOT EXISTS (SELECT 1 FROM locators WHERE resource_id = NEW.resource_id);
END;

CREATE TRIGGER unresolved_memberships_after_membership_delete
AFTER DELETE ON representation_resources
BEGIN
    DELETE FROM unresolved_memberships
    WHERE representation_id = OLD.representation_id AND resource_id = OLD.resource_id;
END;

CREATE TRIGGER unresolved_memberships_after_locator_insert
AFTER INSERT ON locators
BEGIN
    DELETE FROM unresolved_memberships WHERE resource_id = NEW.resource_id;
END;

CREATE TRIGGER unresolved_memberships_after_locator_update
AFTER UPDATE OF resource_id ON locators
BEGIN
    DELETE FROM unresolved_memberships WHERE resource_id = NEW.resource_id;
    INSERT OR IGNORE INTO unresolved_memberships (representation_id, resource_id)
    SELECT rr.representation_id, rr.resource_id
    FROM representation_resources rr
    WHERE rr.resource_id = OLD.resource_id AND rr.required = 1
      AND NOT EXISTS (SELECT 1 FROM locators WHERE resource_id = OLD.resource_id);
END;

CREATE TRIGGER unresolved_memberships_after_locator_delete
AFTER DELETE ON locators
WHEN NOT EXISTS (SELECT 1 FROM locators WHERE resource_id = OLD.resource_id)
BEGIN
    INSERT OR IGNORE INTO unresolved_memberships (representation_id, resource_id)
    SELECT rr.representation_id, rr.resource_id
    FROM representation_resources rr
    WHERE rr.resource_id = OLD.resource_id AND rr.required = 1;
END;

-- Representations reachable through a locator recorded under a logical root.
-- One row per (locator, membership) so pages are keyed by root and
-- representation without reading locators under other roots.
CREATE TABLE media_root_representations (
    media_root_name TEXT NOT NULL
        REFERENCES media_roots(name) ON UPDATE CASCADE ON DELETE CASCADE,
    representation_id BLOB NOT NULL
        REFERENCES representations(id) ON DELETE CASCADE,
    locator_id BLOB NOT NULL REFERENCES locators(id) ON DELETE CASCADE,
    PRIMARY KEY (media_root_name, representation_id, locator_id)
) WITHOUT ROWID;
CREATE INDEX media_root_representations_by_locator
    ON media_root_representations(locator_id);

INSERT OR IGNORE INTO media_root_representations (
    media_root_name, representation_id, locator_id
)
SELECT l.media_root_name, rr.representation_id, l.id
FROM locators l
JOIN representation_resources rr ON rr.resource_id = l.resource_id
WHERE l.media_root_name IS NOT NULL;

CREATE TRIGGER media_root_representations_after_locator_insert
AFTER INSERT ON locators
WHEN NEW.media_root_name IS NOT NULL
BEGIN
    INSERT OR IGNORE INTO media_root_representations (
        media_root_name, representation_id, locator_id
    )
    SELECT NEW.media_root_name, rr.representation_id, NEW.id
    FROM representation_resources rr WHERE rr.resource_id = NEW.resource_id;
END;

CREATE TRIGGER media_root_representations_after_locator_update
AFTER UPDATE OF resource_id, media_root_name ON locators
BEGIN
    DELETE FROM media_root_representations WHERE locator_id = OLD.id;
    INSERT OR IGNORE INTO media_root_representations (
        media_root_name, representation_id, locator_id
    )
    SELECT NEW.media_root_name, rr.representation_id, NEW.id
    FROM representation_resources rr
    WHERE rr.resource_id = NEW.resource_id AND NEW.media_root_name IS NOT NULL;
END;

CREATE TRIGGER media_root_representations_after_membership_insert
AFTER INSERT ON representation_resources
BEGIN
    INSERT OR IGNORE INTO media_root_representations (
        media_root_name, representation_id, locator_id
    )
    SELECT l.media_root_name, NEW.representation_id, l.id
    FROM locators l
    WHERE l.resource_id = NEW.resource_id AND l.media_root_name IS NOT NULL;
END;

CREATE TRIGGER media_root_representations_after_membership_delete
AFTER DELETE ON representation_resources
BEGIN
    DELETE FROM media_root_representations
    WHERE representation_id = OLD.representation_id
      AND locator_id IN (SELECT id FROM locators WHERE resource_id = OLD.resource_id);
END;

CREATE TRIGGER media_root_representations_after_membership_update
AFTER UPDATE OF representation_id, resource_id ON representation_resources
BEGIN
    DELETE FROM media_root_representations
    WHERE representation_id = OLD.representation_id
      AND locator_id IN (SELECT id FROM locators WHERE resource_id = OLD.resource_id);
    INSERT OR IGNORE INTO media_root_representations (
        media_root_name, representation_id, locator_id
    )
    SELECT l.media_root_name, NEW.representation_id, l.id
    FROM locators l
    WHERE l.resource_id = NEW.resource_id AND l.media_root_name IS NOT NULL;
END;

DROP INDEX locators_by_media_root_resource;

-- Activity kind and tool identity copied onto each output edge. Activities are
-- immutable complete facts, so the copy never diverges from its activity.
CREATE TABLE activity_output_keys (
    activity_id BLOB NOT NULL REFERENCES activities(id) ON DELETE CASCADE,
    representation_id BLOB NOT NULL
        REFERENCES representations(id) ON DELETE CASCADE,
    kind TEXT NOT NULL COLLATE BINARY,
    tool_name TEXT COLLATE BINARY,
    tool_version TEXT COLLATE BINARY,
    tool_uri TEXT COLLATE BINARY,
    PRIMARY KEY (activity_id, representation_id)
) WITHOUT ROWID;
CREATE INDEX activity_output_keys_by_kind
    ON activity_output_keys(kind, representation_id);
CREATE INDEX activity_output_keys_by_tool
    ON activity_output_keys(tool_name, tool_version, tool_uri, representation_id)
    WHERE tool_name IS NOT NULL;

INSERT OR IGNORE INTO activity_output_keys (
    activity_id, representation_id, kind, tool_name, tool_version, tool_uri
)
SELECT o.activity_id, o.representation_id, a.kind, a.tool_name, a.tool_version, a.tool_uri
FROM activity_outputs o
JOIN activities a ON a.id = o.activity_id;

CREATE TRIGGER activity_output_keys_after_output_insert
AFTER INSERT ON activity_outputs
BEGIN
    INSERT OR IGNORE INTO activity_output_keys (
        activity_id, representation_id, kind, tool_name, tool_version, tool_uri
    )
    SELECT NEW.activity_id, NEW.representation_id, kind, tool_name, tool_version, tool_uri
    FROM activities WHERE id = NEW.activity_id;
END;

CREATE TRIGGER activity_output_keys_after_output_delete
AFTER DELETE ON activity_outputs
WHEN NOT EXISTS (
    SELECT 1 FROM activity_outputs
    WHERE activity_id = OLD.activity_id AND representation_id = OLD.representation_id
)
BEGIN
    DELETE FROM activity_output_keys
    WHERE activity_id = OLD.activity_id AND representation_id = OLD.representation_id;
END;

DROP INDEX activities_by_tool;

CREATE TRIGGER activity_output_keys_after_activity_update
AFTER UPDATE OF kind, tool_name, tool_version, tool_uri ON activities
BEGIN
    UPDATE activity_output_keys
    SET kind = NEW.kind, tool_name = NEW.tool_name,
        tool_version = NEW.tool_version, tool_uri = NEW.tool_uri
    WHERE activity_id = NEW.id;
END;

UPDATE productions SET schema_version = 12;
