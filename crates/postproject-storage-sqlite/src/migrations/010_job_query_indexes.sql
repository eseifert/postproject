CREATE INDEX jobs_by_kind_id ON jobs(kind, id);

UPDATE productions SET schema_version = 10;
