CREATE TABLE jobs (
    id BLOB PRIMARY KEY CHECK (length(id) = 16),
    kind TEXT NOT NULL COLLATE BINARY
        CHECK (length(CAST(kind AS BLOB)) BETWEEN 1 AND 128),
    output_asset_id BLOB NOT NULL
        REFERENCES assets(id) ON DELETE RESTRICT,
    output_representation_kind INTEGER NOT NULL
        CHECK (output_representation_kind BETWEEN 0 AND 3),
    target_root TEXT COLLATE BINARY CHECK (
        target_root IS NULL OR
        length(CAST(target_root AS BLOB)) BETWEEN 1 AND 128
    ),
    state INTEGER NOT NULL CHECK (state BETWEEN 1 AND 5),
    claim_id BLOB UNIQUE CHECK (claim_id IS NULL OR length(claim_id) = 16),
    claim_tool_name TEXT COLLATE BINARY CHECK (
        claim_tool_name IS NULL OR
        length(CAST(claim_tool_name AS BLOB)) BETWEEN 1 AND 256
    ),
    claim_tool_version TEXT COLLATE BINARY CHECK (
        claim_tool_version IS NULL OR
        length(CAST(claim_tool_version AS BLOB)) BETWEEN 1 AND 128
    ),
    claim_tool_uri TEXT COLLATE BINARY CHECK (
        claim_tool_uri IS NULL OR
        length(CAST(claim_tool_uri AS BLOB)) BETWEEN 1 AND 4096
    ),
    claim_agent_name TEXT COLLATE BINARY CHECK (
        claim_agent_name IS NULL OR
        length(CAST(claim_agent_name AS BLOB)) BETWEEN 1 AND 256
    ),
    claim_agent_scheme TEXT COLLATE BINARY CHECK (
        claim_agent_scheme IS NULL OR
        length(CAST(claim_agent_scheme AS BLOB)) BETWEEN 1 AND 255
    ),
    claim_agent_value TEXT COLLATE BINARY CHECK (
        claim_agent_value IS NULL OR
        length(CAST(claim_agent_value AS BLOB)) BETWEEN 1 AND 4096
    ),
    claim_agent_qualifier TEXT COLLATE BINARY CHECK (
        claim_agent_qualifier IS NULL OR
        length(CAST(claim_agent_qualifier AS BLOB)) BETWEEN 1 AND 1024
    ),
    claim_expires_at_micros INTEGER,
    completion_activity_id BLOB
        REFERENCES activities(id) ON DELETE RESTRICT,
    completion_representation_id BLOB
        REFERENCES representations(id) ON DELETE RESTRICT,
    failure_diagnostic TEXT COLLATE BINARY CHECK (
        failure_diagnostic IS NULL OR
        length(CAST(failure_diagnostic AS BLOB)) BETWEEN 1 AND 4096
    ),
    CHECK (
        (claim_agent_scheme IS NULL AND claim_agent_value IS NULL AND
         claim_agent_qualifier IS NULL) OR
        (claim_agent_scheme IS NOT NULL AND claim_agent_value IS NOT NULL)
    ),
    CHECK (
        (state IN (1, 5) AND claim_id IS NULL AND claim_tool_name IS NULL AND
         claim_tool_version IS NULL AND claim_tool_uri IS NULL AND
         claim_agent_name IS NULL AND claim_agent_scheme IS NULL AND
         claim_agent_value IS NULL AND claim_agent_qualifier IS NULL AND
         claim_expires_at_micros IS NULL AND completion_activity_id IS NULL AND
         completion_representation_id IS NULL AND failure_diagnostic IS NULL) OR
        (state = 2 AND claim_id IS NOT NULL AND claim_tool_name IS NOT NULL AND
         claim_expires_at_micros IS NOT NULL AND completion_activity_id IS NULL AND
         completion_representation_id IS NULL AND failure_diagnostic IS NULL) OR
        (state = 3 AND claim_id IS NULL AND claim_tool_name IS NULL AND
         claim_tool_version IS NULL AND claim_tool_uri IS NULL AND
         claim_agent_name IS NULL AND claim_agent_scheme IS NULL AND
         claim_agent_value IS NULL AND claim_agent_qualifier IS NULL AND
         claim_expires_at_micros IS NULL AND completion_activity_id IS NOT NULL AND
         completion_representation_id IS NOT NULL AND failure_diagnostic IS NULL) OR
        (state = 4 AND claim_id IS NULL AND claim_tool_name IS NULL AND
         claim_tool_version IS NULL AND claim_tool_uri IS NULL AND
         claim_agent_name IS NULL AND claim_agent_scheme IS NULL AND
         claim_agent_value IS NULL AND claim_agent_qualifier IS NULL AND
         claim_expires_at_micros IS NULL AND completion_activity_id IS NULL AND
         completion_representation_id IS NULL AND failure_diagnostic IS NOT NULL)
    )
);

CREATE TABLE job_inputs (
    job_id BLOB NOT NULL REFERENCES jobs(id) ON DELETE CASCADE,
    position INTEGER NOT NULL CHECK (position >= 0),
    representation_id BLOB NOT NULL
        REFERENCES representations(id) ON DELETE RESTRICT,
    PRIMARY KEY (job_id, position),
    UNIQUE (job_id, representation_id)
);

CREATE INDEX jobs_by_state_kind ON jobs(state, kind, id);
CREATE INDEX job_inputs_by_representation
    ON job_inputs(representation_id, job_id, position);

DROP TRIGGER delete_asset_attachments;
DROP TRIGGER delete_representation_attachments;
DROP TRIGGER delete_resource_attachments;
DROP TRIGGER delete_activity_attachments;

ALTER TABLE metadata_assertions RENAME TO metadata_assertions_before_jobs;

-- target_kind: 0 production, 1 asset, 2 representation, 3 resource,
-- 4 activity, 5 job.
CREATE TABLE metadata_assertions (
    id INTEGER PRIMARY KEY,
    target_kind INTEGER NOT NULL CHECK (target_kind BETWEEN 0 AND 5),
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

INSERT INTO metadata_assertions
    (id, target_kind, target_id, vocabulary, property, position, encoded_value)
SELECT id, target_kind, target_id, vocabulary, property, position, encoded_value
FROM metadata_assertions_before_jobs;

DROP TABLE metadata_assertions_before_jobs;

CREATE INDEX metadata_assertions_by_target
    ON metadata_assertions(
        target_kind, target_id, vocabulary, property, position, id
    );
CREATE INDEX metadata_assertions_by_property
    ON metadata_assertions(
        vocabulary, property, target_kind, target_id, position, id
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

CREATE TRIGGER delete_activity_attachments
AFTER DELETE ON activities
BEGIN
    DELETE FROM external_identifiers WHERE target_kind = 4 AND target_id = OLD.id;
    DELETE FROM metadata_assertions WHERE target_kind = 4 AND target_id = OLD.id;
END;

CREATE TRIGGER delete_job_attachments
AFTER DELETE ON jobs
BEGIN
    DELETE FROM metadata_assertions WHERE target_kind = 5 AND target_id = OLD.id;
END;

ALTER TABLE revision_events RENAME TO revision_events_before_jobs;

CREATE TABLE revision_events (
    revision_id BLOB NOT NULL REFERENCES revisions(id) ON DELETE CASCADE,
    position INTEGER NOT NULL CHECK (position >= 0),
    kind INTEGER NOT NULL CHECK (kind BETWEEN 1 AND 26),
    target_kind INTEGER CHECK (target_kind BETWEEN 0 AND 5),
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

INSERT INTO revision_events SELECT * FROM revision_events_before_jobs;
DROP TABLE revision_events_before_jobs;

UPDATE productions SET schema_version = 9;
