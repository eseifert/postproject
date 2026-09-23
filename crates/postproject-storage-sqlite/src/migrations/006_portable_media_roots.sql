ALTER TABLE media_roots RENAME TO media_roots_absolute;

CREATE TABLE media_roots (
    id BLOB PRIMARY KEY CHECK (length(id) = 16),
    name TEXT NOT NULL UNIQUE CHECK (length(name) BETWEEN 1 AND 128),
    label TEXT,
    legacy_uri TEXT UNIQUE,
    priority INTEGER NOT NULL,
    enabled INTEGER NOT NULL CHECK (enabled IN (0, 1))
);

INSERT INTO media_roots (id, name, label, legacy_uri, priority, enabled)
SELECT id, 'legacy-' || lower(hex(id)), label, uri, priority, enabled
FROM media_roots_absolute;

DROP TABLE media_roots_absolute;

CREATE INDEX enabled_media_roots_by_priority
    ON media_roots(priority, id)
    WHERE enabled = 1;

UPDATE productions SET schema_version = 6;
