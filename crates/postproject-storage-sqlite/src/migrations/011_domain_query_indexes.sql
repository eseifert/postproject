ALTER TABLE locators ADD COLUMN media_root_name TEXT
    REFERENCES media_roots(name) ON UPDATE CASCADE ON DELETE SET NULL;

CREATE INDEX assets_by_creation_identity
    ON assets(created_at_micros, id);
CREATE INDEX locators_by_media_root_resource
    ON locators(media_root_name, resource_id, id)
    WHERE media_root_name IS NOT NULL;
CREATE INDEX locators_by_resource_availability
    ON locators(resource_id, availability, id);
CREATE INDEX metadata_assertions_by_property_value
    ON metadata_assertions(
        vocabulary, property, encoded_value, target_kind, target_id, position, id
    );
CREATE INDEX activities_by_tool
    ON activities(tool_name, tool_version, tool_uri, id)
    WHERE tool_name IS NOT NULL;
CREATE INDEX activity_outputs_by_activity_representation
    ON activity_outputs(activity_id, representation_id);
CREATE INDEX revision_events_by_primary_target
    ON revision_events(kind, primary_id, revision_id);
CREATE INDEX revision_events_by_secondary_target
    ON revision_events(kind, secondary_id, revision_id)
    WHERE secondary_id IS NOT NULL;

UPDATE productions SET schema_version = 11;
