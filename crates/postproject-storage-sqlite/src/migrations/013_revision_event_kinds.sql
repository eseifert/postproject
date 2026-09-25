-- Derived key table for event-type-filtered revision pages. A trigger fills it
-- from every inserted event, so no caller maintains it. Keyed by event kind
-- and revision sequence, a filtered page reads at most one page of keys per
-- requested kind instead of scanning the journal.
CREATE TABLE revision_event_kinds (
    kind INTEGER NOT NULL,
    sequence INTEGER NOT NULL
        REFERENCES revisions(sequence) ON DELETE CASCADE,
    PRIMARY KEY (kind, sequence)
) WITHOUT ROWID;
-- Supports the cascade when a revision is deleted.
CREATE INDEX revision_event_kinds_by_sequence
    ON revision_event_kinds(sequence);

INSERT OR IGNORE INTO revision_event_kinds (kind, sequence)
SELECT e.kind, r.sequence
FROM revision_events e
JOIN revisions r ON r.id = e.revision_id;

CREATE TRIGGER revision_event_kinds_after_event_insert
AFTER INSERT ON revision_events
BEGIN
    INSERT OR IGNORE INTO revision_event_kinds (kind, sequence)
    SELECT NEW.kind, sequence FROM revisions WHERE id = NEW.revision_id;
END;

UPDATE productions SET schema_version = 13;
