//! SQLite project-file persistence for `PostProject`.
//!
//! This crate translates between domain values and a private, migrated SQLite
//! schema. SQLite types and errors are never part of the core API contract.

#![forbid(unsafe_code)]

mod metadata_codec;
mod migrations;
mod transaction;

use std::{
    fs::OpenOptions,
    path::{Path, PathBuf},
    time::Duration,
};

use postproject_core::{
    Asset, AssetId, ContentStructure, Error, ErrorKind, ExternalIdentifier, FileFacts, FrameRange,
    IdentifierScheme, ImageSequenceDescriptor, ImageSequencePattern, Locator, LocatorAvailability,
    LocatorId, MediaRoot, MediaRootId, MetadataAssertion, MetadataMatch, MetadataProperty,
    MetadataValue, ObjectRef, Project, ProjectId, ProjectRead, ProjectStore, PropertyId,
    RationalRate, Representation, RepresentationFingerprint, RepresentationId, RepresentationKind,
    Resource, ResourceFingerprint, ResourceId, ResourceMember, ResourceRole, Result, Timestamp,
    VocabularyId,
};
use rusqlite::{Connection, OpenFlags, OptionalExtension, limits::Limit, params};

pub use migrations::CURRENT_SCHEMA_VERSION;
pub use transaction::SqliteTransaction;

const MAX_SQLITE_VALUE_BYTES: i32 = 16 * 1024 * 1024;

/// A project backed by one SQLite project file.
#[derive(Debug)]
pub struct SqliteProject {
    path: PathBuf,
    connection: Connection,
    project: Project,
}

impl SqliteProject {
    /// Creates a new project file and persists its identity atomically.
    ///
    /// # Errors
    ///
    /// Returns [`ErrorKind::AlreadyExists`] if `path` already exists. I/O,
    /// migration, and storage failures are mapped to their domain categories.
    pub fn create(path: impl AsRef<Path>, display_name: Option<String>) -> Result<Self> {
        let path = path.as_ref();
        reserve_new_file(path)?;

        let mut connection = open_connection(path)?;
        migrations::migrate(&mut connection)?;

        let project = Project::new(
            ProjectId::new(),
            CURRENT_SCHEMA_VERSION,
            Timestamp::now()?,
            display_name,
        );
        persist_new_project(&mut connection, &project)?;

        Ok(Self {
            path: path.to_path_buf(),
            connection,
            project,
        })
    }

    /// Opens an existing project file, applying supported migrations first.
    ///
    /// # Errors
    ///
    /// Returns [`ErrorKind::NotFound`] if `path` does not exist. Invalid,
    /// unsupported, or inaccessible databases return a migration or storage error.
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        if !path.is_file() {
            return Err(Error::new(
                ErrorKind::NotFound,
                format!("project file does not exist: {}", path.display()),
            ));
        }

        let mut connection = open_connection(path)?;
        migrations::migrate(&mut connection)?;
        let project = load_project(&connection)?;

        Ok(Self {
            path: path.to_path_buf(),
            connection,
            project,
        })
    }

    /// Returns the project-file path used by this backend.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Returns the loaded project value.
    #[must_use]
    pub const fn project(&self) -> &Project {
        &self.project
    }

    /// Begins an explicit domain transaction for project mutations.
    ///
    /// # Errors
    ///
    /// Returns [`ErrorKind::Storage`] if SQLite cannot start the transaction.
    pub fn begin_transaction(&mut self) -> Result<SqliteTransaction<'_>> {
        let (connection, project) = (&mut self.connection, &mut self.project);
        SqliteTransaction::begin(connection, project)
    }

    /// Loads all assets in deterministic creation/identity order.
    ///
    /// # Errors
    ///
    /// Returns [`ErrorKind::Storage`] for query failures or invalid stored data.
    pub fn assets(&self) -> Result<Vec<Asset>> {
        let mut statement = self
            .connection
            .prepare(
                "SELECT id, created_at_micros, display_name, import_source
                 FROM assets ORDER BY created_at_micros, id",
            )
            .map_err(sqlite_error("prepare asset query"))?;
        let rows = statement
            .query_map([], |row| {
                Ok((
                    row.get::<_, Vec<u8>>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, Option<String>>(3)?,
                ))
            })
            .map_err(sqlite_error("query assets"))?;

        rows.map(|row| {
            let (id, created_at, display_name, import_source) =
                row.map_err(sqlite_error("read asset row"))?;
            Ok(Asset::new(
                AssetId::from_bytes(id_bytes(id, "asset")?),
                Timestamp::from_unix_micros(created_at),
                display_name,
                import_source,
            ))
        })
        .collect()
    }

    /// Loads representations belonging to `asset_id` in stable identity order.
    ///
    /// # Errors
    ///
    /// Returns [`ErrorKind::Storage`] for query failures or invalid stored data.
    pub fn representations(&self, asset_id: AssetId) -> Result<Vec<Representation>> {
        let mut statement = self
            .connection
            .prepare(
                "SELECT id, kind, structure_kind FROM representations
                 WHERE asset_id = ?1 ORDER BY id",
            )
            .map_err(sqlite_error("prepare representation query"))?;
        let rows = statement
            .query_map(params![asset_id.as_bytes().as_slice()], |row| {
                Ok((
                    row.get::<_, Vec<u8>>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, i64>(2)?,
                ))
            })
            .map_err(sqlite_error("query representations"))?;

        rows.map(|row| {
            let (id, kind, structure_kind) =
                row.map_err(sqlite_error("read representation row"))?;
            let id = RepresentationId::from_bytes(id_bytes(id, "representation")?);
            let kind = decode_representation_kind(kind)?;
            let content_structure = self.load_content_structure(id, structure_kind)?;
            let fingerprints = self.load_representation_fingerprints(id)?;
            Ok(Representation::new(
                id,
                asset_id,
                kind,
                content_structure,
                fingerprints,
            ))
        })
        .collect()
    }

    /// Loads resources for `representation_id` in structural order.
    ///
    /// # Errors
    ///
    /// Returns [`ErrorKind::Storage`] for query failures or invalid stored data.
    pub fn resources(&self, representation_id: RepresentationId) -> Result<Vec<Resource>> {
        let mut statement = self
            .connection
            .prepare(
                "SELECT r.id, r.file_size_bytes, r.modified_at_micros
                 FROM representation_resources rr
                 JOIN resources r ON r.id = rr.resource_id
                 WHERE rr.representation_id = ?1 ORDER BY rr.position",
            )
            .map_err(sqlite_error("prepare resource query"))?;
        let rows = statement
            .query_map(params![representation_id.as_bytes().as_slice()], |row| {
                Ok((
                    row.get::<_, Vec<u8>>(0)?,
                    row.get::<_, Option<i64>>(1)?,
                    row.get::<_, Option<i64>>(2)?,
                ))
            })
            .map_err(sqlite_error("query resources"))?;

        rows.map(|row| {
            let (id, size, modified_at) = row.map_err(sqlite_error("read resource row"))?;
            let id = ResourceId::from_bytes(id_bytes(id, "resource")?);
            Ok(Resource::new(
                id,
                self.load_resource_fingerprints(id)?,
                decode_file_facts(size, modified_at)?,
            ))
        })
        .collect()
    }

    /// Loads known locators for `resource_id` in stable identity order.
    ///
    /// # Errors
    ///
    /// Returns [`ErrorKind::Storage`] for query failures or invalid stored data.
    pub fn locators(&self, resource_id: ResourceId) -> Result<Vec<Locator>> {
        let mut statement = self
            .connection
            .prepare(
                "SELECT id, uri, last_seen_micros, availability FROM locators
                 WHERE resource_id = ?1 ORDER BY id",
            )
            .map_err(sqlite_error("prepare locator query"))?;
        let rows = statement
            .query_map(params![resource_id.as_bytes().as_slice()], |row| {
                Ok((
                    row.get::<_, Vec<u8>>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<i64>>(2)?,
                    row.get::<_, i64>(3)?,
                ))
            })
            .map_err(sqlite_error("query locators"))?;

        rows.map(|row| {
            let (id, uri, last_seen, availability) =
                row.map_err(sqlite_error("read locator row"))?;
            Locator::new(
                LocatorId::from_bytes(id_bytes(id, "locator")?),
                resource_id,
                uri,
                last_seen.map(Timestamp::from_unix_micros),
                decode_availability(availability)?,
            )
            .map_err(stored_domain_error("locator"))
        })
        .collect()
    }

    fn load_content_structure(
        &self,
        representation_id: RepresentationId,
        structure_kind: i64,
    ) -> Result<ContentStructure> {
        let mut statement = self
            .connection
            .prepare(
                "SELECT resource_id, role, required
                 FROM representation_resources
                 WHERE representation_id = ?1 ORDER BY position",
            )
            .map_err(sqlite_error("prepare content-membership query"))?;
        let rows = statement
            .query_map(params![representation_id.as_bytes().as_slice()], |row| {
                Ok((
                    row.get::<_, Vec<u8>>(0)?,
                    row.get::<_, Option<String>>(1)?,
                    row.get::<_, bool>(2)?,
                ))
            })
            .map_err(sqlite_error("query content memberships"))?;
        let rows: Vec<_> = rows
            .map(|row| {
                let (id, role, required) =
                    row.map_err(sqlite_error("read content-membership row"))?;
                Ok((
                    ResourceId::from_bytes(id_bytes(id, "resource")?),
                    role,
                    required,
                ))
            })
            .collect::<Result<_>>()?;

        match structure_kind {
            0 => match rows.as_slice() {
                [(resource_id, None, true)] => Ok(ContentStructure::single_resource(*resource_id)),
                _ => Err(stored_invariant("invalid single-resource membership")),
            },
            1 => {
                let resource_id = match rows.as_slice() {
                    [(resource_id, None, true)] => *resource_id,
                    _ => return Err(stored_invariant("invalid image-sequence membership")),
                };
                self.load_image_sequence(representation_id, resource_id)
                    .map(ContentStructure::image_sequence)
            }
            2 | 3 => {
                let members = rows
                    .into_iter()
                    .map(|(resource_id, role, required)| {
                        let role = role.ok_or_else(|| {
                            stored_invariant("compound resource membership has no role")
                        })?;
                        let role = ResourceRole::new(role)
                            .map_err(stored_domain_error("resource role"))?;
                        Ok(ResourceMember::new(resource_id, role, required))
                    })
                    .collect::<Result<_>>()?;
                if structure_kind == 2 {
                    ContentStructure::ordered_parts(members)
                } else {
                    ContentStructure::package(members)
                }
                .map_err(stored_domain_error("content structure"))
            }
            _ => Err(stored_invariant("invalid content-structure kind")),
        }
    }

    fn load_image_sequence(
        &self,
        representation_id: RepresentationId,
        resource_id: ResourceId,
    ) -> Result<ImageSequenceDescriptor> {
        let row = self
            .connection
            .query_row(
                "SELECT resource_id, prefix, suffix, padding, start_frame, end_frame,
                        frame_step, rate_numerator, rate_denominator
                 FROM image_sequences WHERE representation_id = ?1",
                params![representation_id.as_bytes().as_slice()],
                |row| {
                    Ok((
                        row.get::<_, Vec<u8>>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, i64>(3)?,
                        row.get::<_, i64>(4)?,
                        row.get::<_, i64>(5)?,
                        row.get::<_, i64>(6)?,
                        row.get::<_, i64>(7)?,
                        row.get::<_, i64>(8)?,
                    ))
                },
            )
            .map_err(sqlite_error("load image-sequence descriptor"))?;
        let stored_resource = ResourceId::from_bytes(id_bytes(row.0, "sequence resource")?);
        if stored_resource != resource_id {
            return Err(stored_invariant(
                "image-sequence resource does not match membership",
            ));
        }
        let pattern = ImageSequencePattern::new(row.1, row.2, stored_u8(row.3, "padding")?)
            .map_err(stored_domain_error("image-sequence pattern"))?;
        let frames = FrameRange::new(row.4, row.5, stored_u32(row.6, "frame step")?)
            .map_err(stored_domain_error("image-sequence frame range"))?;
        let rate = RationalRate::new(
            stored_u32(row.7, "rate numerator")?,
            stored_u32(row.8, "rate denominator")?,
        )
        .map_err(stored_domain_error("image-sequence rate"))?;
        let mut statement = self
            .connection
            .prepare(
                "SELECT frame FROM image_sequence_missing_frames
                 WHERE representation_id = ?1 ORDER BY frame",
            )
            .map_err(sqlite_error("prepare missing-frame query"))?;
        let missing = statement
            .query_map(params![representation_id.as_bytes().as_slice()], |row| {
                row.get::<_, i64>(0)
            })
            .map_err(sqlite_error("query missing frames"))?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(sqlite_error("read missing-frame row"))?;
        ImageSequenceDescriptor::new(resource_id, pattern, frames, rate, missing)
            .map_err(stored_domain_error("image-sequence descriptor"))
    }

    fn load_resource_fingerprints(
        &self,
        resource_id: ResourceId,
    ) -> Result<Vec<ResourceFingerprint>> {
        let mut statement = self
            .connection
            .prepare(
                "SELECT algorithm, algorithm_version, value
                 FROM resource_fingerprints
                 WHERE resource_id = ?1 ORDER BY algorithm, algorithm_version",
            )
            .map_err(sqlite_error("prepare resource-fingerprint query"))?;
        let rows = statement
            .query_map(params![resource_id.as_bytes().as_slice()], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, u16>(1)?,
                    row.get::<_, Vec<u8>>(2)?,
                ))
            })
            .map_err(sqlite_error("query resource fingerprints"))?;
        rows.map(|row| {
            let (algorithm, version, value) =
                row.map_err(sqlite_error("read resource-fingerprint row"))?;
            ResourceFingerprint::new(algorithm, version, value)
                .map_err(stored_domain_error("resource fingerprint"))
        })
        .collect()
    }

    fn load_representation_fingerprints(
        &self,
        representation_id: RepresentationId,
    ) -> Result<Vec<RepresentationFingerprint>> {
        let mut statement = self
            .connection
            .prepare(
                "SELECT algorithm, algorithm_version, value
                 FROM representation_fingerprints
                 WHERE representation_id = ?1 ORDER BY algorithm, algorithm_version",
            )
            .map_err(sqlite_error("prepare representation-fingerprint query"))?;
        let rows = statement
            .query_map(params![representation_id.as_bytes().as_slice()], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, u16>(1)?,
                    row.get::<_, Vec<u8>>(2)?,
                ))
            })
            .map_err(sqlite_error("query representation fingerprints"))?;
        rows.map(|row| {
            let (algorithm, version, value) =
                row.map_err(sqlite_error("read representation-fingerprint row"))?;
            RepresentationFingerprint::new(algorithm, version, value)
                .map_err(stored_domain_error("representation fingerprint"))
        })
        .collect()
    }

    /// Loads external identifiers attached to `target` in deterministic order.
    ///
    /// # Errors
    ///
    /// Returns [`ErrorKind::Unsupported`] for a target kind not yet persisted,
    /// or [`ErrorKind::Storage`] for query failures and malformed stored data.
    pub fn external_identifiers(&self, target: ObjectRef) -> Result<Vec<ExternalIdentifier>> {
        let (target_kind, target_id) = encode_identifier_target(&target)?;
        let mut statement = self
            .connection
            .prepare(
                "SELECT scheme, value, qualifier
                 FROM external_identifiers
                 WHERE target_kind = ?1 AND target_id = ?2
                 ORDER BY scheme, value, qualifier, id",
            )
            .map_err(sqlite_error("prepare external-identifier query"))?;
        let rows = statement
            .query_map(params![target_kind, target_id.as_slice()], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<String>>(2)?,
                ))
            })
            .map_err(sqlite_error("query external identifiers"))?;

        rows.map(|row| {
            let (scheme, value, qualifier) =
                row.map_err(sqlite_error("read external-identifier row"))?;
            decode_external_identifier(scheme, value, qualifier)
        })
        .collect()
    }

    /// Finds objects carrying an exact external identifier scheme and value.
    ///
    /// Multiple qualifiers on one object produce that object only once.
    ///
    /// # Errors
    ///
    /// Returns [`ErrorKind::InvalidArgument`] for an invalid lookup value or
    /// [`ErrorKind::Storage`] for query failures and malformed target data.
    pub fn find_by_external_identifier(
        &self,
        scheme: &IdentifierScheme,
        value: &str,
    ) -> Result<Vec<ObjectRef>> {
        ExternalIdentifier::new(scheme.clone(), value, None)?;
        let mut statement = self
            .connection
            .prepare(
                "SELECT DISTINCT target_kind, target_id
                 FROM external_identifiers
                 WHERE scheme = ?1 AND value = ?2
                 ORDER BY target_kind, target_id",
            )
            .map_err(sqlite_error("prepare external-identifier lookup"))?;
        let rows = statement
            .query_map(params![scheme.as_str(), value], |row| {
                Ok((row.get::<_, i64>(0)?, row.get::<_, Vec<u8>>(1)?))
            })
            .map_err(sqlite_error("look up external identifier"))?;

        rows.map(|row| {
            let (kind, id) = row.map_err(sqlite_error("read external-identifier target"))?;
            decode_identifier_target(kind, id)
        })
        .collect()
    }

    /// Loads metadata attached to `target`, grouped by property and ordered by
    /// each value's insertion position.
    ///
    /// # Errors
    ///
    /// Returns [`ErrorKind::Storage`] for query failures or malformed encoded
    /// values, or [`ErrorKind::Unsupported`] for an unknown target kind.
    pub fn metadata(&self, target: ObjectRef) -> Result<Vec<MetadataAssertion>> {
        let (target_kind, target_id) = encode_metadata_target(&target)?;
        let mut statement = self
            .connection
            .prepare(
                "SELECT vocabulary, property, encoded_value
                 FROM metadata_assertions
                 WHERE target_kind = ?1 AND target_id = ?2
                 ORDER BY vocabulary, property, position, id",
            )
            .map_err(sqlite_error("prepare metadata query"))?;
        let rows = statement
            .query_map(params![target_kind, target_id.as_slice()], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Vec<u8>>(2)?,
                ))
            })
            .map_err(sqlite_error("query metadata"))?;

        rows.map(|row| {
            let (vocabulary, property, encoded) = row.map_err(sqlite_error("read metadata row"))?;
            decode_metadata_assertion(vocabulary, property, &encoded)
        })
        .collect()
    }

    /// Loads ordered repeated values for `property` on `target`.
    ///
    /// # Errors
    ///
    /// Returns [`ErrorKind::Storage`] for query failures or malformed encoded
    /// values, or [`ErrorKind::Unsupported`] for an unknown target kind.
    pub fn metadata_values(
        &self,
        target: ObjectRef,
        property: &MetadataProperty,
    ) -> Result<Vec<MetadataValue>> {
        let (target_kind, target_id) = encode_metadata_target(&target)?;
        let mut statement = self
            .connection
            .prepare(
                "SELECT encoded_value
                 FROM metadata_assertions
                 WHERE target_kind = ?1 AND target_id = ?2
                   AND vocabulary = ?3 AND property = ?4
                 ORDER BY position, id",
            )
            .map_err(sqlite_error("prepare metadata-value query"))?;
        let rows = statement
            .query_map(
                params![
                    target_kind,
                    target_id.as_slice(),
                    property.vocabulary().as_str(),
                    property.property().as_str(),
                ],
                |row| row.get::<_, Vec<u8>>(0),
            )
            .map_err(sqlite_error("query metadata values"))?;

        rows.map(|row| {
            let encoded = row.map_err(sqlite_error("read metadata-value row"))?;
            metadata_codec::decode(&encoded)
        })
        .collect()
    }

    /// Finds every assertion using `property` in stable target/value order.
    ///
    /// # Errors
    ///
    /// Returns [`ErrorKind::Storage`] for query failures, malformed targets, or
    /// malformed encoded values.
    pub fn query_by_metadata_property(
        &self,
        property: &MetadataProperty,
    ) -> Result<Vec<MetadataMatch>> {
        let mut statement = self
            .connection
            .prepare(
                "SELECT target_kind, target_id, encoded_value
                 FROM metadata_assertions
                 WHERE vocabulary = ?1 AND property = ?2
                 ORDER BY target_kind, target_id, position, id",
            )
            .map_err(sqlite_error("prepare metadata-property lookup"))?;
        let rows = statement
            .query_map(
                params![property.vocabulary().as_str(), property.property().as_str(),],
                |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        row.get::<_, Vec<u8>>(1)?,
                        row.get::<_, Vec<u8>>(2)?,
                    ))
                },
            )
            .map_err(sqlite_error("query metadata property"))?;

        rows.map(|row| {
            let (kind, id, encoded) = row.map_err(sqlite_error("read metadata match"))?;
            let target = decode_metadata_target(kind, id)?;
            let value = metadata_codec::decode(&encoded)?;
            Ok(MetadataMatch::new(
                target,
                MetadataAssertion::new(property.clone(), value),
            ))
        })
        .collect()
    }

    /// Reports whether SQLite foreign-key enforcement is active on this connection.
    ///
    /// This is primarily useful for diagnostics and integration tests.
    ///
    /// # Errors
    ///
    /// Returns [`ErrorKind::Storage`] if SQLite cannot query the setting.
    pub fn foreign_keys_enabled(&self) -> Result<bool> {
        self.connection
            .query_row("PRAGMA foreign_keys", [], |row| row.get::<_, bool>(0))
            .map_err(sqlite_error("query foreign-key enforcement"))
    }
}

impl ProjectRead for SqliteProject {
    fn project(&self) -> &Project {
        SqliteProject::project(self)
    }

    fn assets(&self) -> Result<Vec<Asset>> {
        SqliteProject::assets(self)
    }

    fn representations(&self, asset_id: AssetId) -> Result<Vec<Representation>> {
        SqliteProject::representations(self, asset_id)
    }

    fn resources(&self, representation_id: RepresentationId) -> Result<Vec<Resource>> {
        SqliteProject::resources(self, representation_id)
    }

    fn locators(&self, resource_id: ResourceId) -> Result<Vec<Locator>> {
        SqliteProject::locators(self, resource_id)
    }

    fn external_identifiers(&self, target: ObjectRef) -> Result<Vec<ExternalIdentifier>> {
        SqliteProject::external_identifiers(self, target)
    }

    fn find_by_external_identifier(
        &self,
        scheme: &IdentifierScheme,
        value: &str,
    ) -> Result<Vec<ObjectRef>> {
        SqliteProject::find_by_external_identifier(self, scheme, value)
    }

    fn metadata(&self, target: ObjectRef) -> Result<Vec<MetadataAssertion>> {
        SqliteProject::metadata(self, target)
    }

    fn metadata_values(
        &self,
        target: ObjectRef,
        property: &MetadataProperty,
    ) -> Result<Vec<MetadataValue>> {
        SqliteProject::metadata_values(self, target, property)
    }

    fn query_by_metadata_property(
        &self,
        property: &MetadataProperty,
    ) -> Result<Vec<MetadataMatch>> {
        SqliteProject::query_by_metadata_property(self, property)
    }
}

impl ProjectStore for SqliteProject {
    type Transaction<'project> = SqliteTransaction<'project>;

    fn begin_transaction(&mut self) -> Result<Self::Transaction<'_>> {
        SqliteProject::begin_transaction(self)
    }
}

fn reserve_new_file(path: &Path) -> Result<()> {
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map(|_| ())
        .map_err(|error| {
            let kind = if error.kind() == std::io::ErrorKind::AlreadyExists {
                ErrorKind::AlreadyExists
            } else {
                ErrorKind::Io
            };
            Error::new(
                kind,
                format!("cannot create project file {}: {error}", path.display()),
            )
        })
}

fn open_connection(path: &Path) -> Result<Connection> {
    let connection = Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(sqlite_error("open project database"))?;
    configure_length_limit(&connection, MAX_SQLITE_VALUE_BYTES)?;
    connection
        .busy_timeout(Duration::from_secs(5))
        .map_err(sqlite_error("configure SQLite busy timeout"))?;
    connection
        .execute_batch("PRAGMA foreign_keys = ON; PRAGMA trusted_schema = OFF;")
        .map_err(sqlite_error("configure SQLite connection"))?;
    Ok(connection)
}

fn configure_length_limit(connection: &Connection, maximum: i32) -> Result<()> {
    connection
        .set_limit(Limit::SQLITE_LIMIT_LENGTH, maximum)
        .map(|_| ())
        .map_err(sqlite_error("configure SQLite value length limit"))
}

fn persist_new_project(connection: &mut Connection, project: &Project) -> Result<()> {
    let transaction = connection
        .transaction()
        .map_err(sqlite_error("begin project creation"))?;
    transaction
        .execute(
            "INSERT INTO projects (
                singleton, id, schema_version, created_at_micros, display_name
             ) VALUES (1, ?1, ?2, ?3, ?4)",
            params![
                project.id().as_bytes().as_slice(),
                project.schema_version(),
                project.created_at().as_unix_micros(),
                project.display_name(),
            ],
        )
        .map_err(sqlite_error("persist new project"))?;
    transaction
        .commit()
        .map_err(sqlite_error("commit project creation"))
}

fn load_project(connection: &Connection) -> Result<Project> {
    let stored = connection
        .query_row(
            "SELECT id, schema_version, created_at_micros, display_name
             FROM projects WHERE singleton = 1",
            [],
            |row| {
                Ok((
                    row.get::<_, Vec<u8>>(0)?,
                    row.get::<_, u32>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, Option<String>>(3)?,
                ))
            },
        )
        .optional()
        .map_err(sqlite_error("load project record"))?
        .ok_or_else(|| Error::new(ErrorKind::Storage, "database has no project record"))?;

    let id = ProjectId::from_bytes(id_bytes(stored.0, "project")?);
    if stored.1 != CURRENT_SCHEMA_VERSION {
        return Err(Error::new(
            ErrorKind::Migration,
            format!(
                "project record schema version {} does not match database version {}",
                stored.1, CURRENT_SCHEMA_VERSION
            ),
        ));
    }

    let mut project = Project::new(
        id,
        stored.1,
        Timestamp::from_unix_micros(stored.2),
        stored.3,
    );
    project.set_media_roots(load_media_roots(connection)?);
    Ok(project)
}

fn load_media_roots(connection: &Connection) -> Result<Vec<MediaRoot>> {
    let mut statement = connection
        .prepare("SELECT id, uri, label, priority, enabled FROM media_roots ORDER BY priority, id")
        .map_err(sqlite_error("prepare media-root query"))?;
    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, Vec<u8>>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, i32>(3)?,
                row.get::<_, bool>(4)?,
            ))
        })
        .map_err(sqlite_error("query media roots"))?;
    rows.map(|row| {
        let (id, uri, label, priority, enabled) =
            row.map_err(sqlite_error("read media-root row"))?;
        MediaRoot::new(
            MediaRootId::from_bytes(id_bytes(id, "media root")?),
            uri,
            label,
            priority,
            enabled,
        )
        .map_err(stored_domain_error("media root"))
    })
    .collect()
}

fn decode_representation_kind(value: i64) -> Result<RepresentationKind> {
    match value {
        0 => Ok(RepresentationKind::Original),
        1 => Ok(RepresentationKind::Proxy),
        2 => Ok(RepresentationKind::Optimized),
        3 => Ok(RepresentationKind::Derived),
        _ => Err(Error::new(
            ErrorKind::Storage,
            format!("stored representation kind {value} is invalid"),
        )),
    }
}

fn decode_availability(value: i64) -> Result<LocatorAvailability> {
    match value {
        0 => Ok(LocatorAvailability::Unknown),
        1 => Ok(LocatorAvailability::Online),
        2 => Ok(LocatorAvailability::Offline),
        _ => Err(Error::new(
            ErrorKind::Storage,
            format!("stored locator availability {value} is invalid"),
        )),
    }
}

fn decode_file_facts(size: Option<i64>, modified_at: Option<i64>) -> Result<Option<FileFacts>> {
    match (size, modified_at) {
        (None, None) => Ok(None),
        (Some(size), modified_at) => {
            let size = u64::try_from(size).map_err(|error| {
                Error::new(
                    ErrorKind::Storage,
                    format!("stored file size is invalid: {error}"),
                )
            })?;
            Ok(Some(FileFacts::new(
                size,
                modified_at.map(Timestamp::from_unix_micros),
            )))
        }
        (None, Some(_)) => Err(Error::new(
            ErrorKind::Storage,
            "stored modification time has no corresponding file size",
        )),
    }
}

fn decode_external_identifier(
    scheme: String,
    value: String,
    qualifier: Option<String>,
) -> Result<ExternalIdentifier> {
    let scheme = IdentifierScheme::new(scheme).map_err(stored_domain_error("identifier scheme"))?;
    ExternalIdentifier::new(scheme, value, qualifier)
        .map_err(stored_domain_error("external identifier"))
}

fn decode_metadata_assertion(
    vocabulary: String,
    property: String,
    encoded: &[u8],
) -> Result<MetadataAssertion> {
    let vocabulary =
        VocabularyId::new(vocabulary).map_err(stored_domain_error("metadata vocabulary"))?;
    let property = PropertyId::new(property).map_err(stored_domain_error("metadata property"))?;
    let value = metadata_codec::decode(encoded)?;
    Ok(MetadataAssertion::new(
        MetadataProperty::new(vocabulary, property),
        value,
    ))
}

pub(crate) fn encode_identifier_target(target: &ObjectRef) -> Result<(i64, &[u8; 16])> {
    match target {
        ObjectRef::Asset(id) => Ok((1, id.as_bytes())),
        ObjectRef::Representation(id) => Ok((2, id.as_bytes())),
        ObjectRef::Project(_) | ObjectRef::Activity(_) => Err(Error::new(
            ErrorKind::Unsupported,
            "external identifiers currently support assets and representations",
        )),
        _ => Err(Error::new(
            ErrorKind::Unsupported,
            "external identifier target kind is not supported by this schema",
        )),
    }
}

fn decode_identifier_target(kind: i64, id: Vec<u8>) -> Result<ObjectRef> {
    let id = id_bytes(id, "external identifier target")?;
    match kind {
        1 => Ok(ObjectRef::Asset(AssetId::from_bytes(id))),
        2 => Ok(ObjectRef::Representation(RepresentationId::from_bytes(id))),
        _ => Err(Error::new(
            ErrorKind::Storage,
            format!("stored external identifier target kind {kind} is invalid"),
        )),
    }
}

pub(crate) fn encode_metadata_target(target: &ObjectRef) -> Result<(i64, &[u8; 16])> {
    match target {
        ObjectRef::Project(id) => Ok((0, id.as_bytes())),
        ObjectRef::Asset(id) => Ok((1, id.as_bytes())),
        ObjectRef::Representation(id) => Ok((2, id.as_bytes())),
        ObjectRef::Activity(id) => Ok((3, id.as_bytes())),
        _ => Err(Error::new(
            ErrorKind::Unsupported,
            "metadata target kind is not supported by this schema",
        )),
    }
}

fn decode_metadata_target(kind: i64, id: Vec<u8>) -> Result<ObjectRef> {
    let id = id_bytes(id, "metadata target")?;
    match kind {
        0 => Ok(ObjectRef::Project(ProjectId::from_bytes(id))),
        1 => Ok(ObjectRef::Asset(AssetId::from_bytes(id))),
        2 => Ok(ObjectRef::Representation(RepresentationId::from_bytes(id))),
        3 => Ok(ObjectRef::Activity(
            postproject_core::ActivityId::from_bytes(id),
        )),
        _ => Err(Error::new(
            ErrorKind::Storage,
            format!("stored metadata target kind {kind} is invalid"),
        )),
    }
}

pub(crate) fn id_bytes(value: Vec<u8>, label: &str) -> Result<[u8; 16]> {
    value.try_into().map_err(|value: Vec<u8>| {
        Error::new(
            ErrorKind::Storage,
            format!("stored {label} ID has {} bytes; expected 16", value.len()),
        )
    })
}

fn stored_u32(value: i64, label: &str) -> Result<u32> {
    u32::try_from(value).map_err(|error| {
        Error::new(
            ErrorKind::Storage,
            format!("stored {label} is invalid: {error}"),
        )
    })
}

fn stored_u8(value: i64, label: &str) -> Result<u8> {
    u8::try_from(value).map_err(|error| {
        Error::new(
            ErrorKind::Storage,
            format!("stored {label} is invalid: {error}"),
        )
    })
}

fn stored_invariant(message: &'static str) -> Error {
    Error::new(ErrorKind::Storage, message)
}

pub(crate) fn sqlite_error(context: &'static str) -> impl FnOnce(rusqlite::Error) -> Error {
    move |error| Error::new(ErrorKind::Storage, format!("{context}: {error}"))
}

fn stored_domain_error(label: &'static str) -> impl FnOnce(Error) -> Error {
    move |error| {
        Error::new(
            ErrorKind::Storage,
            format!("stored {label} is invalid: {error}"),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sqlite_value_limit_rejects_oversized_results() {
        let connection = Connection::open_in_memory().expect("open in-memory database");
        configure_length_limit(&connection, 1_024).expect("configure test limit");

        let result =
            connection.query_row("SELECT zeroblob(1025)", [], |row| row.get::<_, Vec<u8>>(0));

        assert!(result.is_err(), "oversized value unexpectedly loaded");
    }
}
