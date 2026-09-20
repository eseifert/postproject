CREATE TABLE activities (
    id BLOB PRIMARY KEY CHECK (length(id) = 16),
    kind TEXT NOT NULL COLLATE BINARY
        CHECK (length(CAST(kind AS BLOB)) BETWEEN 1 AND 128),
    started_at_micros INTEGER,
    finished_at_micros INTEGER,
    tool_name TEXT COLLATE BINARY
        CHECK (
            tool_name IS NULL OR
            length(CAST(tool_name AS BLOB)) BETWEEN 1 AND 256
        ),
    tool_version TEXT COLLATE BINARY
        CHECK (
            tool_version IS NULL OR
            length(CAST(tool_version AS BLOB)) BETWEEN 1 AND 128
        ),
    tool_uri TEXT COLLATE BINARY
        CHECK (
            tool_uri IS NULL OR
            length(CAST(tool_uri AS BLOB)) BETWEEN 1 AND 4096
        ),
    agent_name TEXT COLLATE BINARY
        CHECK (
            agent_name IS NULL OR
            length(CAST(agent_name AS BLOB)) BETWEEN 1 AND 256
        ),
    agent_identifier_scheme TEXT COLLATE BINARY
        CHECK (
            agent_identifier_scheme IS NULL OR
            length(CAST(agent_identifier_scheme AS BLOB)) BETWEEN 1 AND 255
        ),
    agent_identifier_value TEXT COLLATE BINARY
        CHECK (
            agent_identifier_value IS NULL OR
            length(CAST(agent_identifier_value AS BLOB)) BETWEEN 1 AND 4096
        ),
    agent_identifier_qualifier TEXT COLLATE BINARY
        CHECK (
            agent_identifier_qualifier IS NULL OR
            length(CAST(agent_identifier_qualifier AS BLOB)) BETWEEN 1 AND 1024
        ),
    CHECK (
        started_at_micros IS NULL OR
        finished_at_micros IS NULL OR
        finished_at_micros >= started_at_micros
    ),
    CHECK (
        tool_name IS NOT NULL OR
        (tool_version IS NULL AND tool_uri IS NULL)
    ),
    CHECK (
        (agent_identifier_scheme IS NULL) =
        (agent_identifier_value IS NULL)
    ),
    CHECK (
        agent_identifier_qualifier IS NULL OR
        agent_identifier_scheme IS NOT NULL
    )
);

CREATE INDEX activities_by_kind ON activities(kind, id);

CREATE TABLE activity_inputs (
    id INTEGER PRIMARY KEY,
    activity_id BLOB NOT NULL REFERENCES activities(id) ON DELETE CASCADE,
    representation_id BLOB NOT NULL
        REFERENCES representations(id) ON DELETE RESTRICT,
    role TEXT COLLATE BINARY
        CHECK (
            role IS NULL OR
            length(CAST(role AS BLOB)) BETWEEN 1 AND 128
        )
);

CREATE UNIQUE INDEX activity_inputs_unique_edge
    ON activity_inputs(activity_id, representation_id, ifnull(role, ''));
CREATE INDEX activity_inputs_by_representation
    ON activity_inputs(representation_id, activity_id);

CREATE TABLE activity_outputs (
    id INTEGER PRIMARY KEY,
    activity_id BLOB NOT NULL REFERENCES activities(id) ON DELETE CASCADE,
    representation_id BLOB NOT NULL
        REFERENCES representations(id) ON DELETE RESTRICT,
    role TEXT COLLATE BINARY
        CHECK (
            role IS NULL OR
            length(CAST(role AS BLOB)) BETWEEN 1 AND 128
        )
);

CREATE UNIQUE INDEX activity_outputs_unique_edge
    ON activity_outputs(activity_id, representation_id, ifnull(role, ''));
CREATE INDEX activity_outputs_by_representation
    ON activity_outputs(representation_id, activity_id);

CREATE TRIGGER delete_activity_attachments
AFTER DELETE ON activities
BEGIN
    DELETE FROM metadata_assertions WHERE target_kind = 4 AND target_id = OLD.id;
END;

UPDATE productions SET schema_version = 2;
