//! SQLite production-file persistence for `PostProject`.
//!
//! This crate translates between domain values and a private, migrated SQLite
//! schema. SQLite types and errors are never part of the core API contract.

#![forbid(unsafe_code)]

mod artifact;
mod metadata_codec;
mod migrations;
mod transaction;

use std::{
    collections::BTreeMap,
    fs::OpenOptions,
    path::{Path, PathBuf},
    time::Duration,
};

use postproject_core::{
    Activity, ActivityEdgeSnapshot, ActivityId, ActivityInput, ActivityKind, ActivityOutput,
    ActivityRole, AgentIdentity, ArtifactEvaluation, ArtifactEvaluationLimits,
    ArtifactReproducibilityReport, Asset, AssetId, ContentStructure, Dependency, DependencyKind,
    DependencySet, DependencySetStatus, DependencyTarget, Error, ErrorKind, ExternalIdentifier,
    FileFacts, FingerprintSnapshot, FrameRange, IdentifierScheme, ImageSequenceDescriptor,
    ImageSequencePattern, Locator, LocatorAvailability, LocatorId, MAX_REVISION_PAGE_SIZE,
    MediaRoot, MediaRootId, MetadataAssertion, MetadataMatch, MetadataProperty, MetadataValue,
    ObjectRef, OriginIdentity, Production, ProductionId, ProductionRead, ProductionStore,
    PropertyId, RationalRate, Representation, RepresentationFingerprint, RepresentationId,
    RepresentationKind, Resource, ResourceFingerprint, ResourceId, ResourceMember, ResourceRole,
    Result, Revision, RevisionEvent, RevisionEventKind, RevisionId, Timestamp, ToolIdentity,
    TransactionId, VocabularyId,
};
use rusqlite::{Connection, OpenFlags, OptionalExtension, limits::Limit, params};

pub use migrations::CURRENT_SCHEMA_VERSION;
pub use transaction::SqliteTransaction;

const MAX_SQLITE_VALUE_BYTES: i32 = 16 * 1024 * 1024;

/// A production backed by one SQLite production file.
#[derive(Debug)]
pub struct SqliteProduction {
    path: PathBuf,
    connection: Connection,
    production: Production,
}

struct StoredActivity {
    id: Vec<u8>,
    kind: String,
    started_at: Option<i64>,
    finished_at: Option<i64>,
    tool_name: Option<String>,
    tool_version: Option<String>,
    tool_uri: Option<String>,
    agent_name: Option<String>,
    agent_scheme: Option<String>,
    agent_value: Option<String>,
    agent_qualifier: Option<String>,
}

struct StoredRevision {
    id: Vec<u8>,
    sequence: i64,
    transaction_id: Vec<u8>,
    committed_at: i64,
    origin_name: Option<String>,
    origin_version: Option<String>,
    origin_uri: Option<String>,
    message: Option<String>,
}

struct StoredRevisionEvent {
    position: i64,
    kind: i64,
    target_kind: Option<i64>,
    primary_id: Option<Vec<u8>>,
    secondary_id: Option<Vec<u8>>,
    structural_position: Option<i64>,
    vocabulary: Option<String>,
    property: Option<String>,
    identifier_scheme: Option<String>,
    identifier_value: Option<String>,
    identifier_qualifier: Option<String>,
    activity_kind: Option<String>,
    role: Option<String>,
    fingerprint_algorithm: Option<String>,
    fingerprint_version: Option<i64>,
}

struct StoredActivityEdge {
    id: i64,
    representation_id: RepresentationId,
    role: Option<ActivityRole>,
    snapshot_revision_sequence: Option<i64>,
}
type ActivityEdgesById<Edge> = BTreeMap<ActivityId, Vec<Edge>>;

impl SqliteProduction {
    /// Creates a new production file and persists its identity atomically.
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

        let production = Production::new(
            ProductionId::new(),
            CURRENT_SCHEMA_VERSION,
            Timestamp::now()?,
            display_name,
        );
        persist_new_production(&mut connection, &production)?;

        Ok(Self {
            path: path.to_path_buf(),
            connection,
            production,
        })
    }

    /// Opens an existing production file, applying supported migrations first.
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
                format!("production file does not exist: {}", path.display()),
            ));
        }

        let mut connection = open_connection(path)?;
        migrations::migrate(&mut connection)?;
        let production = load_production(&connection)?;

        Ok(Self {
            path: path.to_path_buf(),
            connection,
            production,
        })
    }

    /// Returns the production-file path used by this backend.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Returns the loaded production value.
    #[must_use]
    pub const fn production(&self) -> &Production {
        &self.production
    }

    /// Begins an explicit domain transaction for production mutations.
    ///
    /// # Errors
    ///
    /// Returns [`ErrorKind::Storage`] if SQLite cannot start the transaction.
    pub fn begin_transaction(&mut self) -> Result<SqliteTransaction<'_>> {
        let (connection, production) = (&mut self.connection, &mut self.production);
        SqliteTransaction::begin(connection, production)
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

    fn load_representation_by_id(
        &self,
        representation_id: RepresentationId,
    ) -> Result<Representation> {
        let stored = self
            .connection
            .query_row(
                "SELECT asset_id, kind, structure_kind FROM representations WHERE id = ?1",
                params![representation_id.as_bytes().as_slice()],
                |row| {
                    Ok((
                        row.get::<_, Vec<u8>>(0)?,
                        row.get::<_, i64>(1)?,
                        row.get::<_, i64>(2)?,
                    ))
                },
            )
            .optional()
            .map_err(sqlite_error("load representation"))?
            .ok_or_else(|| Error::new(ErrorKind::NotFound, "representation does not exist"))?;
        let asset_id = AssetId::from_bytes(id_bytes(stored.0, "asset")?);
        Ok(Representation::new(
            representation_id,
            asset_id,
            decode_representation_kind(stored.1)?,
            self.load_content_structure(representation_id, stored.2)?,
            self.load_representation_fingerprints(representation_id)?,
        ))
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

    /// Loads all production activities in stable identity order.
    ///
    /// Activity edges are loaded in two set-oriented queries rather than one
    /// query per activity.
    ///
    /// # Errors
    ///
    /// Returns [`ErrorKind::Storage`] for query failures, malformed activity
    /// values, invalid edges, or orphaned edge rows.
    pub fn activities(&self) -> Result<Vec<Activity>> {
        let mut statement = self
            .connection
            .prepare(
                "SELECT id, kind, started_at_micros, finished_at_micros,
                        tool_name, tool_version, tool_uri, agent_name,
                        agent_identifier_scheme, agent_identifier_value,
                        agent_identifier_qualifier
                 FROM activities ORDER BY id",
            )
            .map_err(sqlite_error("prepare activity query"))?;
        let rows = statement
            .query_map([], |row| {
                Ok(StoredActivity {
                    id: row.get(0)?,
                    kind: row.get(1)?,
                    started_at: row.get(2)?,
                    finished_at: row.get(3)?,
                    tool_name: row.get(4)?,
                    tool_version: row.get(5)?,
                    tool_uri: row.get(6)?,
                    agent_name: row.get(7)?,
                    agent_scheme: row.get(8)?,
                    agent_value: row.get(9)?,
                    agent_qualifier: row.get(10)?,
                })
            })
            .map_err(sqlite_error("query activities"))?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(sqlite_error("read activity row"))?;
        let mut inputs = self.load_activity_inputs()?;
        let mut outputs = self.load_activity_outputs()?;
        let activities = rows
            .into_iter()
            .map(|stored| {
                let id = ActivityId::from_bytes(id_bytes(stored.id.clone(), "activity")?);
                decode_activity(
                    id,
                    stored,
                    inputs.remove(&id).unwrap_or_default(),
                    outputs.remove(&id).unwrap_or_default(),
                )
            })
            .collect::<Result<Vec<_>>>()?;
        if !inputs.is_empty() || !outputs.is_empty() {
            return Err(stored_invariant(
                "activity edge refers to an absent activity",
            ));
        }
        Ok(activities)
    }

    /// Loads activities that produce `representation_id` in stable order.
    ///
    /// # Errors
    ///
    /// Returns [`ErrorKind::NotFound`] when the representation is absent, or
    /// [`ErrorKind::Storage`] when activity data cannot be decoded safely.
    pub fn activities_producing(
        &self,
        representation_id: RepresentationId,
    ) -> Result<Vec<Activity>> {
        self.ensure_representation_exists(representation_id)?;
        Ok(self
            .activities()?
            .into_iter()
            .filter(|activity| {
                activity
                    .outputs()
                    .iter()
                    .any(|output| output.representation_id() == representation_id)
            })
            .collect())
    }

    /// Loads activities that consume `representation_id` in stable order.
    ///
    /// # Errors
    ///
    /// Returns [`ErrorKind::NotFound`] when the representation is absent, or
    /// [`ErrorKind::Storage`] when activity data cannot be decoded safely.
    pub fn activities_consuming(
        &self,
        representation_id: RepresentationId,
    ) -> Result<Vec<Activity>> {
        self.ensure_representation_exists(representation_id)?;
        Ok(self
            .activities()?
            .into_iter()
            .filter(|activity| {
                activity
                    .inputs()
                    .iter()
                    .any(|input| input.representation_id() == representation_id)
            })
            .collect())
    }

    /// Returns transitive input ancestry in stable identity order.
    ///
    /// # Errors
    ///
    /// Returns [`ErrorKind::NotFound`] when the representation is absent, or
    /// [`ErrorKind::Storage`] when traversal fails or stored IDs are malformed.
    pub fn ancestors(&self, representation_id: RepresentationId) -> Result<Vec<RepresentationId>> {
        self.related_representations(
            representation_id,
            "WITH RECURSIVE related(representation_id) AS (
                SELECT inputs.representation_id
                FROM activity_outputs outputs
                JOIN activity_inputs inputs ON inputs.activity_id = outputs.activity_id
                WHERE outputs.representation_id = ?1
                UNION
                SELECT inputs.representation_id
                FROM related
                JOIN activity_outputs outputs
                  ON outputs.representation_id = related.representation_id
                JOIN activity_inputs inputs ON inputs.activity_id = outputs.activity_id
             )
             SELECT representation_id FROM related ORDER BY representation_id",
            "ancestor",
        )
    }

    /// Returns transitive output descendants in stable identity order.
    ///
    /// # Errors
    ///
    /// Returns [`ErrorKind::NotFound`] when the representation is absent, or
    /// [`ErrorKind::Storage`] when traversal fails or stored IDs are malformed.
    pub fn descendants(
        &self,
        representation_id: RepresentationId,
    ) -> Result<Vec<RepresentationId>> {
        self.related_representations(
            representation_id,
            "WITH RECURSIVE related(representation_id) AS (
                SELECT outputs.representation_id
                FROM activity_inputs inputs
                JOIN activity_outputs outputs ON outputs.activity_id = inputs.activity_id
                WHERE inputs.representation_id = ?1
                UNION
                SELECT outputs.representation_id
                FROM related
                JOIN activity_inputs inputs
                  ON inputs.representation_id = related.representation_id
                JOIN activity_outputs outputs ON outputs.activity_id = inputs.activity_id
             )
             SELECT representation_id FROM related ORDER BY representation_id",
            "descendant",
        )
    }

    /// Loads the complete dependency observation for `representation_id`.
    ///
    /// # Errors
    ///
    /// Returns [`ErrorKind::NotFound`] when the source representation is absent,
    /// or a storage-domain error when persisted dependency data is malformed.
    pub fn dependency_set(
        &self,
        representation_id: RepresentationId,
    ) -> Result<Option<DependencySet>> {
        self.ensure_representation_exists(representation_id)?;
        load_dependency_set(&self.connection, representation_id)
    }

    /// Returns the newest durable revision, if the journal is non-empty.
    ///
    /// # Errors
    ///
    /// Returns [`ErrorKind::Storage`] when persisted revision data is malformed
    /// or cannot be read.
    pub fn latest_revision(&self) -> Result<Option<Revision>> {
        self.connection
            .query_row(
                "SELECT id, sequence, transaction_id, committed_at_micros,
                        origin_name, origin_version, origin_uri, message
                 FROM revisions ORDER BY sequence DESC LIMIT 1",
                [],
                stored_revision_row,
            )
            .optional()
            .map_err(sqlite_error("query latest revision"))?
            .map(decode_revision)
            .transpose()
    }

    /// Returns a bounded ascending page of revisions after `sequence`.
    ///
    /// # Errors
    ///
    /// Returns [`ErrorKind::InvalidArgument`] when `limit` is zero or exceeds
    /// [`MAX_REVISION_PAGE_SIZE`], or [`ErrorKind::Storage`] for invalid data.
    pub fn changes_since(&self, sequence: u64, limit: u32) -> Result<Vec<Revision>> {
        if limit == 0 || limit > MAX_REVISION_PAGE_SIZE {
            return Err(Error::new(
                ErrorKind::InvalidArgument,
                format!("revision page limit must be 1-{MAX_REVISION_PAGE_SIZE}"),
            ));
        }
        let Ok(sequence) = i64::try_from(sequence) else {
            return Ok(Vec::new());
        };
        let mut statement = self
            .connection
            .prepare(
                "SELECT id, sequence, transaction_id, committed_at_micros,
                        origin_name, origin_version, origin_uri, message
                 FROM revisions WHERE sequence > ?1
                 ORDER BY sequence LIMIT ?2",
            )
            .map_err(sqlite_error("prepare revision page query"))?;
        statement
            .query_map(params![sequence, i64::from(limit)], stored_revision_row)
            .map_err(sqlite_error("query revision page"))?
            .map(|row| {
                row.map_err(sqlite_error("read revision row"))
                    .and_then(decode_revision)
            })
            .collect()
    }

    /// Loads one revision's semantic events in stable position order.
    ///
    /// # Errors
    ///
    /// Returns [`ErrorKind::NotFound`] when `revision_id` is absent, or
    /// [`ErrorKind::Storage`] when persisted event data is malformed.
    pub fn events_for_revision(&self, revision_id: RevisionId) -> Result<Vec<RevisionEvent>> {
        let exists = self
            .connection
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM revisions WHERE id = ?1)",
                [revision_id.as_bytes().as_slice()],
                |row| row.get::<_, bool>(0),
            )
            .map_err(sqlite_error("check revision existence"))?;
        if !exists {
            return Err(Error::new(ErrorKind::NotFound, "revision does not exist"));
        }

        let mut statement = self
            .connection
            .prepare(
                "SELECT position, kind, target_kind, primary_id, secondary_id,
                        structural_position, vocabulary, property,
                        identifier_scheme, identifier_value, identifier_qualifier,
                        activity_kind, role, fingerprint_algorithm, fingerprint_version
                 FROM revision_events WHERE revision_id = ?1 ORDER BY position",
            )
            .map_err(sqlite_error("prepare revision event query"))?;
        statement
            .query_map(
                [revision_id.as_bytes().as_slice()],
                stored_revision_event_row,
            )
            .map_err(sqlite_error("query revision events"))?
            .map(|row| {
                row.map_err(sqlite_error("read revision event row"))
                    .and_then(|event| decode_revision_event(revision_id, event))
            })
            .collect()
    }

    fn related_representations(
        &self,
        representation_id: RepresentationId,
        query: &'static str,
        label: &'static str,
    ) -> Result<Vec<RepresentationId>> {
        self.ensure_representation_exists(representation_id)?;
        let mut statement = self
            .connection
            .prepare(query)
            .map_err(sqlite_error("prepare provenance traversal"))?;
        let rows = statement
            .query_map([representation_id.as_bytes().as_slice()], |row| {
                row.get::<_, Vec<u8>>(0)
            })
            .map_err(sqlite_error("query provenance traversal"))?;
        rows.map(|row| {
            let id = row.map_err(sqlite_error("read provenance traversal row"))?;
            Ok(RepresentationId::from_bytes(id_bytes(id, label)?))
        })
        .collect()
    }

    fn ensure_representation_exists(&self, representation_id: RepresentationId) -> Result<()> {
        let exists = self
            .connection
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM representations WHERE id = ?1)",
                [representation_id.as_bytes().as_slice()],
                |row| row.get::<_, bool>(0),
            )
            .map_err(sqlite_error("check provenance representation"))?;
        if !exists {
            return Err(Error::new(
                ErrorKind::NotFound,
                "provenance representation does not exist",
            ));
        }
        Ok(())
    }

    fn load_activity_inputs(&self) -> Result<ActivityEdgesById<ActivityInput>> {
        let snapshots = load_activity_edge_snapshots(
            &self.connection,
            "SELECT activity_input_id, algorithm, algorithm_version, value,
                    observed_revision_sequence
             FROM activity_input_fingerprint_snapshots
             ORDER BY activity_input_id, algorithm, algorithm_version",
        )?;
        load_activity_edges(
            &self.connection,
            "SELECT id, activity_id, representation_id, role, snapshot_revision_sequence
             FROM activity_inputs
             ORDER BY activity_id, representation_id, role, id",
        )?
        .into_iter()
        .try_fold(
            ActivityEdgesById::<ActivityInput>::new(),
            |mut result, (activity_id, edge)| {
                let mut input = ActivityInput::new(edge.representation_id, edge.role);
                if let Some(sequence) = edge.snapshot_revision_sequence {
                    input = input.with_snapshot(ActivityEdgeSnapshot::new(
                        stored_u64(sequence, "activity input snapshot revision")?,
                        snapshots.get(&edge.id).cloned().unwrap_or_default(),
                    )?);
                }
                result.entry(activity_id).or_default().push(input);
                Ok(result)
            },
        )
    }

    fn load_activity_outputs(&self) -> Result<ActivityEdgesById<ActivityOutput>> {
        let snapshots = load_activity_edge_snapshots(
            &self.connection,
            "SELECT activity_output_id, algorithm, algorithm_version, value,
                    observed_revision_sequence
             FROM activity_output_fingerprint_snapshots
             ORDER BY activity_output_id, algorithm, algorithm_version",
        )?;
        load_activity_edges(
            &self.connection,
            "SELECT id, activity_id, representation_id, role, snapshot_revision_sequence
             FROM activity_outputs
             ORDER BY activity_id, representation_id, role, id",
        )?
        .into_iter()
        .try_fold(
            ActivityEdgesById::<ActivityOutput>::new(),
            |mut result, (activity_id, edge)| {
                let mut output = ActivityOutput::new(edge.representation_id, edge.role);
                if let Some(sequence) = edge.snapshot_revision_sequence {
                    output = output.with_snapshot(ActivityEdgeSnapshot::new(
                        stored_u64(sequence, "activity output snapshot revision")?,
                        snapshots.get(&edge.id).cloned().unwrap_or_default(),
                    )?);
                }
                result.entry(activity_id).or_default().push(output);
                Ok(result)
            },
        )
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

impl ProductionRead for SqliteProduction {
    fn production(&self) -> &Production {
        SqliteProduction::production(self)
    }

    fn assets(&self) -> Result<Vec<Asset>> {
        SqliteProduction::assets(self)
    }

    fn representations(&self, asset_id: AssetId) -> Result<Vec<Representation>> {
        SqliteProduction::representations(self, asset_id)
    }

    fn resources(&self, representation_id: RepresentationId) -> Result<Vec<Resource>> {
        SqliteProduction::resources(self, representation_id)
    }

    fn locators(&self, resource_id: ResourceId) -> Result<Vec<Locator>> {
        SqliteProduction::locators(self, resource_id)
    }

    fn external_identifiers(&self, target: ObjectRef) -> Result<Vec<ExternalIdentifier>> {
        SqliteProduction::external_identifiers(self, target)
    }

    fn find_by_external_identifier(
        &self,
        scheme: &IdentifierScheme,
        value: &str,
    ) -> Result<Vec<ObjectRef>> {
        SqliteProduction::find_by_external_identifier(self, scheme, value)
    }

    fn metadata(&self, target: ObjectRef) -> Result<Vec<MetadataAssertion>> {
        SqliteProduction::metadata(self, target)
    }

    fn metadata_values(
        &self,
        target: ObjectRef,
        property: &MetadataProperty,
    ) -> Result<Vec<MetadataValue>> {
        SqliteProduction::metadata_values(self, target, property)
    }

    fn query_by_metadata_property(
        &self,
        property: &MetadataProperty,
    ) -> Result<Vec<MetadataMatch>> {
        SqliteProduction::query_by_metadata_property(self, property)
    }

    fn activities(&self) -> Result<Vec<Activity>> {
        SqliteProduction::activities(self)
    }

    fn activities_producing(&self, representation_id: RepresentationId) -> Result<Vec<Activity>> {
        SqliteProduction::activities_producing(self, representation_id)
    }

    fn activities_consuming(&self, representation_id: RepresentationId) -> Result<Vec<Activity>> {
        SqliteProduction::activities_consuming(self, representation_id)
    }

    fn ancestors(&self, representation_id: RepresentationId) -> Result<Vec<RepresentationId>> {
        SqliteProduction::ancestors(self, representation_id)
    }

    fn descendants(&self, representation_id: RepresentationId) -> Result<Vec<RepresentationId>> {
        SqliteProduction::descendants(self, representation_id)
    }

    fn dependency_set(&self, representation_id: RepresentationId) -> Result<Option<DependencySet>> {
        SqliteProduction::dependency_set(self, representation_id)
    }

    fn evaluate_artifact(
        &self,
        representation_id: RepresentationId,
        limits: ArtifactEvaluationLimits,
    ) -> Result<ArtifactEvaluation> {
        SqliteProduction::evaluate_artifact(self, representation_id, limits)
    }

    fn artifact_reproducibility(
        &self,
        representation_id: RepresentationId,
    ) -> Result<ArtifactReproducibilityReport> {
        SqliteProduction::artifact_reproducibility(self, representation_id)
    }

    fn latest_revision(&self) -> Result<Option<Revision>> {
        SqliteProduction::latest_revision(self)
    }

    fn changes_since(&self, sequence: u64, limit: u32) -> Result<Vec<Revision>> {
        SqliteProduction::changes_since(self, sequence, limit)
    }

    fn events_for_revision(&self, revision_id: RevisionId) -> Result<Vec<RevisionEvent>> {
        SqliteProduction::events_for_revision(self, revision_id)
    }
}

impl ProductionStore for SqliteProduction {
    type Transaction<'production> = SqliteTransaction<'production>;

    fn begin_transaction(&mut self) -> Result<Self::Transaction<'_>> {
        SqliteProduction::begin_transaction(self)
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
                format!("cannot create production file {}: {error}", path.display()),
            )
        })
}

fn open_connection(path: &Path) -> Result<Connection> {
    let connection = Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(sqlite_error("open production database"))?;
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

fn persist_new_production(connection: &mut Connection, production: &Production) -> Result<()> {
    let transaction = connection
        .transaction()
        .map_err(sqlite_error("begin production creation"))?;
    transaction
        .execute(
            "INSERT INTO productions (
                singleton, id, schema_version, created_at_micros, display_name
             ) VALUES (1, ?1, ?2, ?3, ?4)",
            params![
                production.id().as_bytes().as_slice(),
                production.schema_version(),
                production.created_at().as_unix_micros(),
                production.display_name(),
            ],
        )
        .map_err(sqlite_error("persist new production"))?;
    transaction
        .commit()
        .map_err(sqlite_error("commit production creation"))
}

fn load_production(connection: &Connection) -> Result<Production> {
    let stored = connection
        .query_row(
            "SELECT id, schema_version, created_at_micros, display_name
             FROM productions WHERE singleton = 1",
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
        .map_err(sqlite_error("load production record"))?
        .ok_or_else(|| Error::new(ErrorKind::Storage, "database has no production record"))?;

    let id = ProductionId::from_bytes(id_bytes(stored.0, "production")?);
    if stored.1 != CURRENT_SCHEMA_VERSION {
        return Err(Error::new(
            ErrorKind::Migration,
            format!(
                "production record schema version {} does not match database version {}",
                stored.1, CURRENT_SCHEMA_VERSION
            ),
        ));
    }

    let mut production = Production::new(
        id,
        stored.1,
        Timestamp::from_unix_micros(stored.2),
        stored.3,
    );
    production.set_media_roots(load_media_roots(connection)?);
    Ok(production)
}

fn load_media_roots(connection: &Connection) -> Result<Vec<MediaRoot>> {
    let mut statement = connection
        .prepare(
            "SELECT id, name, label, legacy_uri, priority, enabled
             FROM media_roots ORDER BY priority, id",
        )
        .map_err(sqlite_error("prepare media-root query"))?;
    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, Vec<u8>>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, Option<String>>(3)?,
                row.get::<_, i32>(4)?,
                row.get::<_, bool>(5)?,
            ))
        })
        .map_err(sqlite_error("query media roots"))?;
    rows.map(|row| {
        let (id, name, label, legacy_uri, priority, enabled) =
            row.map_err(sqlite_error("read media-root row"))?;
        MediaRoot::new(
            MediaRootId::from_bytes(id_bytes(id, "media root")?),
            name,
            label,
            legacy_uri,
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

fn load_activity_edges(
    connection: &Connection,
    query: &'static str,
) -> Result<Vec<(ActivityId, StoredActivityEdge)>> {
    let mut statement = connection
        .prepare(query)
        .map_err(sqlite_error("prepare activity-edge query"))?;
    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, Vec<u8>>(1)?,
                row.get::<_, Vec<u8>>(2)?,
                row.get::<_, Option<String>>(3)?,
                row.get::<_, Option<i64>>(4)?,
            ))
        })
        .map_err(sqlite_error("query activity edges"))?;
    rows.map(|row| {
        let (edge_id, activity_id, representation_id, role, snapshot_revision_sequence) =
            row.map_err(sqlite_error("read activity-edge row"))?;
        let activity_id = ActivityId::from_bytes(id_bytes(activity_id, "activity edge")?);
        let representation_id =
            RepresentationId::from_bytes(id_bytes(representation_id, "activity representation")?);
        let role = role
            .map(ActivityRole::new)
            .transpose()
            .map_err(stored_domain_error("activity role"))?;
        Ok((
            activity_id,
            StoredActivityEdge {
                id: edge_id,
                representation_id,
                role,
                snapshot_revision_sequence,
            },
        ))
    })
    .collect()
}

fn load_activity_edge_snapshots(
    connection: &Connection,
    query: &'static str,
) -> Result<BTreeMap<i64, Vec<FingerprintSnapshot>>> {
    let mut statement = connection
        .prepare(query)
        .map_err(sqlite_error("prepare activity snapshot query"))?;
    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, i64>(2)?,
                row.get::<_, Vec<u8>>(3)?,
                row.get::<_, Option<i64>>(4)?,
            ))
        })
        .map_err(sqlite_error("query activity snapshots"))?;
    rows.map(|row| {
        let (edge_id, algorithm, version, value, observed_sequence) =
            row.map_err(sqlite_error("read activity snapshot row"))?;
        let version = u16::try_from(version)
            .map_err(|_| stored_invariant("activity snapshot version is outside u16"))?;
        let observed_sequence = observed_sequence
            .map(|sequence| stored_u64(sequence, "fingerprint observation revision"))
            .transpose()?;
        let fingerprint = FingerprintSnapshot::new(algorithm, version, value, observed_sequence)
            .map_err(stored_domain_error("activity fingerprint snapshot"))?;
        Ok((edge_id, fingerprint))
    })
    .try_fold(
        BTreeMap::<i64, Vec<FingerprintSnapshot>>::new(),
        |mut result, row| {
            let (edge_id, fingerprint) = row?;
            result.entry(edge_id).or_default().push(fingerprint);
            Ok(result)
        },
    )
}

fn decode_activity(
    id: ActivityId,
    stored: StoredActivity,
    inputs: Vec<ActivityInput>,
    outputs: Vec<ActivityOutput>,
) -> Result<Activity> {
    let kind = ActivityKind::new(stored.kind).map_err(stored_domain_error("activity kind"))?;
    let mut activity = Activity::new(id, kind, inputs, outputs)
        .map_err(stored_domain_error("activity edges"))?
        .with_timing(
            stored.started_at.map(Timestamp::from_unix_micros),
            stored.finished_at.map(Timestamp::from_unix_micros),
        )
        .map_err(stored_domain_error("activity timing"))?;
    match (stored.tool_name, stored.tool_version, stored.tool_uri) {
        (Some(name), version, uri) => {
            activity = activity.with_tool(
                ToolIdentity::new(name, version, uri)
                    .map_err(stored_domain_error("activity tool"))?,
            );
        }
        (None, None, None) => {}
        (None, _, _) => return Err(stored_invariant("activity tool detail has no name")),
    }
    let identifier = match (
        stored.agent_scheme,
        stored.agent_value,
        stored.agent_qualifier,
    ) {
        (Some(scheme), Some(value), qualifier) => {
            Some(decode_external_identifier(scheme, value, qualifier)?)
        }
        (None, None, None) => None,
        _ => return Err(stored_invariant("activity agent identifier is incomplete")),
    };
    if stored.agent_name.is_some() || identifier.is_some() {
        activity = activity.with_agent(
            AgentIdentity::new(stored.agent_name, identifier)
                .map_err(stored_domain_error("activity agent"))?,
        );
    }
    Ok(activity)
}

fn stored_revision_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<StoredRevision> {
    Ok(StoredRevision {
        id: row.get(0)?,
        sequence: row.get(1)?,
        transaction_id: row.get(2)?,
        committed_at: row.get(3)?,
        origin_name: row.get(4)?,
        origin_version: row.get(5)?,
        origin_uri: row.get(6)?,
        message: row.get(7)?,
    })
}

fn decode_revision(stored: StoredRevision) -> Result<Revision> {
    let id = RevisionId::from_bytes(id_bytes(stored.id, "revision")?);
    let sequence = u64::try_from(stored.sequence)
        .map_err(|_| stored_invariant("revision sequence is negative"))?;
    let transaction_id =
        TransactionId::from_bytes(id_bytes(stored.transaction_id, "revision transaction")?);
    let origin = match (stored.origin_name, stored.origin_version, stored.origin_uri) {
        (Some(name), version, uri) => Some(
            OriginIdentity::new(name, version, uri)
                .map_err(stored_domain_error("revision origin"))?,
        ),
        (None, None, None) => None,
        (None, _, _) => return Err(stored_invariant("revision origin detail has no name")),
    };
    Revision::new(
        id,
        sequence,
        transaction_id,
        Timestamp::from_unix_micros(stored.committed_at),
        origin,
        stored.message,
    )
    .map_err(stored_domain_error("revision"))
}

fn stored_revision_event_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<StoredRevisionEvent> {
    Ok(StoredRevisionEvent {
        position: row.get(0)?,
        kind: row.get(1)?,
        target_kind: row.get(2)?,
        primary_id: row.get(3)?,
        secondary_id: row.get(4)?,
        structural_position: row.get(5)?,
        vocabulary: row.get(6)?,
        property: row.get(7)?,
        identifier_scheme: row.get(8)?,
        identifier_value: row.get(9)?,
        identifier_qualifier: row.get(10)?,
        activity_kind: row.get(11)?,
        role: row.get(12)?,
        fingerprint_algorithm: row.get(13)?,
        fingerprint_version: row.get(14)?,
    })
}

#[allow(
    clippy::too_many_lines,
    reason = "keeping the exhaustive semantic-event schema mapping together is auditable"
)]
fn decode_revision_event(
    revision_id: RevisionId,
    stored: StoredRevisionEvent,
) -> Result<RevisionEvent> {
    let position = stored_u32(stored.position, "revision event position")?;
    let primary_id = |label| required_stored_id(stored.primary_id.clone(), label);
    let secondary_id = |label| required_stored_id(stored.secondary_id.clone(), label);
    let target = || {
        let kind = required_stored(stored.target_kind, "revision event target kind")?;
        decode_metadata_target(
            kind,
            required_stored(stored.primary_id.clone(), "target ID")?,
        )
    };
    let identifier = || {
        decode_external_identifier(
            required_stored(
                stored.identifier_scheme.clone(),
                "revision event identifier scheme",
            )?,
            required_stored(
                stored.identifier_value.clone(),
                "revision event identifier value",
            )?,
            stored.identifier_qualifier.clone(),
        )
    };
    let property = || {
        let vocabulary = VocabularyId::new(required_stored(
            stored.vocabulary.clone(),
            "revision event vocabulary",
        )?)
        .map_err(stored_domain_error("revision event vocabulary"))?;
        let property = PropertyId::new(required_stored(
            stored.property.clone(),
            "revision event property",
        )?)
        .map_err(stored_domain_error("revision event property"))?;
        Ok(MetadataProperty::new(vocabulary, property))
    };

    let kind = match stored.kind {
        1 => RevisionEventKind::AssetImported {
            asset_id: AssetId::from_bytes(primary_id("asset")?),
        },
        2 => RevisionEventKind::RepresentationAdded {
            asset_id: AssetId::from_bytes(secondary_id("asset")?),
            representation_id: RepresentationId::from_bytes(primary_id("representation")?),
        },
        3 => RevisionEventKind::ResourceAdded {
            resource_id: ResourceId::from_bytes(primary_id("resource")?),
        },
        4 => RevisionEventKind::RepresentationResourceAdded {
            representation_id: RepresentationId::from_bytes(primary_id("representation")?),
            resource_id: ResourceId::from_bytes(secondary_id("resource")?),
            position: stored_u32(
                required_stored(
                    stored.structural_position,
                    "revision event structural position",
                )?,
                "revision event structural position",
            )?,
        },
        5 => RevisionEventKind::LocatorAdded {
            resource_id: ResourceId::from_bytes(secondary_id("resource")?),
            locator_id: LocatorId::from_bytes(primary_id("locator")?),
        },
        6 => RevisionEventKind::MediaRootAdded {
            media_root_id: MediaRootId::from_bytes(primary_id("media root")?),
        },
        14 => RevisionEventKind::LocatorRetired {
            resource_id: ResourceId::from_bytes(secondary_id("resource")?),
            locator_id: LocatorId::from_bytes(primary_id("locator")?),
        },
        15 => RevisionEventKind::MediaRootEnabledChanged {
            media_root_id: MediaRootId::from_bytes(primary_id("media root")?),
            enabled: match required_stored(
                stored.structural_position,
                "revision event enabled state",
            )? {
                0 => false,
                1 => true,
                value => {
                    return Err(Error::new(
                        ErrorKind::Storage,
                        format!("stored media-root enabled state {value} is invalid"),
                    ));
                }
            },
        },
        16 => RevisionEventKind::MediaRootRemoved {
            media_root_id: MediaRootId::from_bytes(primary_id("media root")?),
        },
        7 => RevisionEventKind::ExternalIdentifierAdded {
            target: target()?,
            identifier: identifier()?,
        },
        8 => RevisionEventKind::ExternalIdentifierRemoved {
            target: target()?,
            identifier: identifier()?,
        },
        9 => RevisionEventKind::MetadataAddedOrReplaced {
            target: target()?,
            property: property()?,
        },
        10 => RevisionEventKind::MetadataRemoved {
            target: target()?,
            property: property()?,
        },
        11 => RevisionEventKind::ActivityCreated {
            activity_id: ActivityId::from_bytes(primary_id("activity")?),
            kind: ActivityKind::new(required_stored(
                stored.activity_kind,
                "revision event activity kind",
            )?)
            .map_err(stored_domain_error("revision event activity kind"))?,
        },
        12 | 13 => {
            let activity_id = ActivityId::from_bytes(primary_id("activity")?);
            let representation_id = RepresentationId::from_bytes(secondary_id("representation")?);
            let role = stored
                .role
                .map(ActivityRole::new)
                .transpose()
                .map_err(stored_domain_error("revision event role"))?;
            if stored.kind == 12 {
                RevisionEventKind::ActivityInputAdded {
                    activity_id,
                    representation_id,
                    role,
                }
            } else {
                RevisionEventKind::ActivityOutputAdded {
                    activity_id,
                    representation_id,
                    role,
                }
            }
        }
        17 | 18 => {
            let algorithm = required_stored(
                stored.fingerprint_algorithm,
                "revision event fingerprint algorithm",
            )?;
            let version = u16::try_from(required_stored(
                stored.fingerprint_version,
                "revision event fingerprint version",
            )?)
            .map_err(|_| stored_invariant("revision event fingerprint version is invalid"))?;
            ResourceFingerprint::new(algorithm.clone(), version, vec![1])
                .map_err(stored_domain_error("revision event fingerprint domain"))?;
            if stored.kind == 17 {
                RevisionEventKind::ResourceFingerprintObserved {
                    resource_id: ResourceId::from_bytes(primary_id("resource")?),
                    algorithm,
                    version,
                }
            } else {
                RevisionEventKind::RepresentationFingerprintObserved {
                    representation_id: RepresentationId::from_bytes(primary_id("representation")?),
                    algorithm,
                    version,
                }
            }
        }
        19 => RevisionEventKind::DependencySetRecorded {
            representation_id: RepresentationId::from_bytes(primary_id("representation")?),
        },
        kind => {
            return Err(Error::new(
                ErrorKind::Storage,
                format!("stored revision event kind {kind} is invalid"),
            ));
        }
    };
    Ok(RevisionEvent::new(revision_id, position, kind))
}

fn required_stored<T>(value: Option<T>, label: &'static str) -> Result<T> {
    value.ok_or_else(|| Error::new(ErrorKind::Storage, format!("stored {label} is missing")))
}

fn required_stored_id(value: Option<Vec<u8>>, label: &'static str) -> Result<[u8; 16]> {
    id_bytes(required_stored(value, label)?, label)
}

pub(crate) fn encode_identifier_target(target: &ObjectRef) -> Result<(i64, &[u8; 16])> {
    match target {
        ObjectRef::Asset(id) => Ok((1, id.as_bytes())),
        ObjectRef::Representation(id) => Ok((2, id.as_bytes())),
        ObjectRef::Resource(id) => Ok((3, id.as_bytes())),
        ObjectRef::Activity(id) => Ok((4, id.as_bytes())),
        ObjectRef::Production(_) => Err(Error::new(
            ErrorKind::Unsupported,
            "external identifiers do not support productions",
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
        3 => Ok(ObjectRef::Resource(ResourceId::from_bytes(id))),
        4 => Ok(ObjectRef::Activity(
            postproject_core::ActivityId::from_bytes(id),
        )),
        _ => Err(Error::new(
            ErrorKind::Storage,
            format!("stored external identifier target kind {kind} is invalid"),
        )),
    }
}

pub(crate) fn encode_metadata_target(target: &ObjectRef) -> Result<(i64, &[u8; 16])> {
    match target {
        ObjectRef::Production(id) => Ok((0, id.as_bytes())),
        ObjectRef::Asset(id) => Ok((1, id.as_bytes())),
        ObjectRef::Representation(id) => Ok((2, id.as_bytes())),
        ObjectRef::Resource(id) => Ok((3, id.as_bytes())),
        ObjectRef::Activity(id) => Ok((4, id.as_bytes())),
        _ => Err(Error::new(
            ErrorKind::Unsupported,
            "metadata target kind is not supported by this schema",
        )),
    }
}

fn decode_metadata_target(kind: i64, id: Vec<u8>) -> Result<ObjectRef> {
    let id = id_bytes(id, "metadata target")?;
    match kind {
        0 => Ok(ObjectRef::Production(ProductionId::from_bytes(id))),
        1 => Ok(ObjectRef::Asset(AssetId::from_bytes(id))),
        2 => Ok(ObjectRef::Representation(RepresentationId::from_bytes(id))),
        3 => Ok(ObjectRef::Resource(ResourceId::from_bytes(id))),
        4 => Ok(ObjectRef::Activity(
            postproject_core::ActivityId::from_bytes(id),
        )),
        _ => Err(Error::new(
            ErrorKind::Storage,
            format!("stored metadata target kind {kind} is invalid"),
        )),
    }
}

fn decode_dependency(
    source_resource: Option<Vec<u8>>,
    kind: String,
    target_kind: i64,
    target: Vec<u8>,
    resolved: Option<Vec<u8>>,
    required: i64,
    authored_reference: String,
) -> Result<Dependency> {
    let source_resource_id = source_resource
        .map(|value| id_bytes(value, "dependency source resource").map(ResourceId::from_bytes))
        .transpose()?;
    let target = id_bytes(target, "dependency target")?;
    let target = match target_kind {
        1 => DependencyTarget::Asset(AssetId::from_bytes(target)),
        2 => DependencyTarget::Representation(RepresentationId::from_bytes(target)),
        _ => return Err(stored_invariant("dependency target kind is invalid")),
    };
    let resolved_representation_id = resolved
        .map(|value| {
            id_bytes(value, "resolved dependency representation").map(RepresentationId::from_bytes)
        })
        .transpose()?;
    let required = match required {
        0 => false,
        1 => true,
        _ => return Err(stored_invariant("dependency requiredness is invalid")),
    };
    Dependency::new(
        source_resource_id,
        DependencyKind::new(kind).map_err(stored_domain_error("dependency kind"))?,
        target,
        resolved_representation_id,
        required,
        authored_reference,
    )
    .map_err(stored_domain_error("dependency"))
}

pub(crate) fn load_dependency_set(
    connection: &Connection,
    representation_id: RepresentationId,
) -> Result<Option<DependencySet>> {
    let header = connection
        .query_row(
            "SELECT recorded_revision_sequence, needs_extraction
             FROM dependency_sets WHERE source_representation_id = ?1",
            [representation_id.as_bytes().as_slice()],
            |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
        )
        .optional()
        .map_err(sqlite_error("query dependency-set observation"))?;
    let Some((revision, needs_extraction)) = header else {
        return Ok(None);
    };
    let mut statement = connection
        .prepare(
            "SELECT source_resource_id, kind, target_kind, target_id,
                    resolved_representation_id, required, authored_reference
             FROM dependencies WHERE source_representation_id = ?1
             ORDER BY position",
        )
        .map_err(sqlite_error("prepare dependency query"))?;
    let dependencies = statement
        .query_map([representation_id.as_bytes().as_slice()], |row| {
            Ok((
                row.get::<_, Option<Vec<u8>>>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, i64>(2)?,
                row.get::<_, Vec<u8>>(3)?,
                row.get::<_, Option<Vec<u8>>>(4)?,
                row.get::<_, i64>(5)?,
                row.get::<_, String>(6)?,
            ))
        })
        .map_err(sqlite_error("query dependencies"))?
        .map(|row| {
            let (source_resource, kind, target_kind, target, resolved, required, authored) =
                row.map_err(sqlite_error("read dependency row"))?;
            decode_dependency(
                source_resource,
                kind,
                target_kind,
                target,
                resolved,
                required,
                authored,
            )
        })
        .collect::<Result<Vec<_>>>()?;
    let status = match needs_extraction {
        0 => DependencySetStatus::Current,
        1 => DependencySetStatus::NeedsExtraction,
        _ => return Err(stored_invariant("dependency-set status is invalid")),
    };
    DependencySet::new(
        representation_id,
        stored_u64(revision, "dependency-set revision")?,
        status,
        dependencies,
    )
    .map(Some)
    .map_err(stored_domain_error("dependency set"))
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

fn stored_u64(value: i64, label: &str) -> Result<u64> {
    u64::try_from(value).map_err(|error| {
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
