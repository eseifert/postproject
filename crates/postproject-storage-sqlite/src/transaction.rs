//! Explicit SQLite-backed domain transactions.

use postproject_core::{
    ContentStructure, ContentStructureKind, Error, ErrorKind, ExternalIdentifier, Locator,
    LocatorAvailability, MediaRoot, MetadataProperty, MetadataValue, ObjectRef,
    OriginalMediaImport, Project, ProjectStoreTransaction, Resource, Result, TransactionId,
    TransactionLifecycle, TransactionState,
};
use rusqlite::{Connection, ErrorCode, Transaction, TransactionBehavior, params};

use crate::{encode_identifier_target, encode_metadata_target, metadata_codec, sqlite_error};

/// An explicit project mutation transaction.
///
/// Dropping an open value rolls its SQLite transaction back. Call [`Self::commit`]
/// to make all staged mutations durable or [`Self::rollback`] to discard them
/// explicitly.
pub struct SqliteTransaction<'project> {
    transaction: Option<Transaction<'project>>,
    lifecycle: TransactionLifecycle,
    project: &'project mut Project,
    pending_roots: Vec<MediaRoot>,
}

impl<'project> SqliteTransaction<'project> {
    pub(crate) fn begin(
        connection: &'project mut Connection,
        project: &'project mut Project,
    ) -> Result<Self> {
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Deferred)
            .map_err(sqlite_error("begin domain transaction"))?;
        Ok(Self {
            transaction: Some(transaction),
            lifecycle: TransactionLifecycle::new(),
            project,
            pending_roots: Vec::new(),
        })
    }

    /// Returns the stable transaction identity.
    #[must_use]
    pub const fn id(&self) -> TransactionId {
        self.lifecycle.id()
    }

    /// Returns the transaction lifecycle state.
    #[must_use]
    pub const fn state(&self) -> TransactionState {
        self.lifecycle.state()
    }

    /// Stages a prepared original-media import atomically.
    ///
    /// # Errors
    ///
    /// Returns [`ErrorKind::Conflict`] if the transaction is closed,
    /// [`ErrorKind::Unsupported`] if the file size exceeds SQLite's signed
    /// integer range, or a storage/conflict error if persistence fails.
    pub fn import_original(&mut self, import: &OriginalMediaImport) -> Result<()> {
        let transaction = self.open_transaction()?;
        let asset = import.asset();
        let representation = import.representation();

        transaction
            .execute(
                "INSERT INTO assets (id, created_at_micros, display_name, import_source)
                 VALUES (?1, ?2, ?3, ?4)",
                params![
                    asset.id().as_bytes().as_slice(),
                    asset.created_at().as_unix_micros(),
                    asset.display_name(),
                    asset.import_source(),
                ],
            )
            .map_err(mutation_error("persist imported asset"))?;
        transaction
            .execute(
                "INSERT INTO representations (
                    id, asset_id, kind, structure_kind
                 ) VALUES (?1, ?2, 0, ?3)",
                params![
                    representation.id().as_bytes().as_slice(),
                    asset.id().as_bytes().as_slice(),
                    encode_structure_kind(representation.content_structure().kind())?,
                ],
            )
            .map_err(mutation_error("persist original representation"))?;
        for fingerprint in representation.fingerprints() {
            transaction
                .execute(
                    "INSERT INTO representation_fingerprints (
                        representation_id, algorithm, algorithm_version, value
                     ) VALUES (?1, ?2, ?3, ?4)",
                    params![
                        representation.id().as_bytes().as_slice(),
                        fingerprint.algorithm(),
                        fingerprint.version(),
                        fingerprint.value(),
                    ],
                )
                .map_err(mutation_error("persist representation fingerprint"))?;
        }
        for resource in import.resources() {
            persist_resource(transaction, resource)?;
        }
        persist_content_structure(
            transaction,
            representation.id(),
            representation.content_structure(),
        )?;
        for locator in import.locators() {
            persist_locator(transaction, locator)?;
        }
        Ok(())
    }

    /// Stages an additional confirmed locator for a resource.
    ///
    /// # Errors
    ///
    /// Returns [`ErrorKind::Conflict`] if the transaction is closed,
    /// [`ErrorKind::AlreadyExists`] for a duplicate locator or missing owning
    /// resource, or [`ErrorKind::Storage`] for other persistence failures.
    pub fn add_locator(&mut self, locator: &Locator) -> Result<()> {
        persist_locator(self.open_transaction()?, locator)
    }

    /// Stages a configured media root.
    ///
    /// # Errors
    ///
    /// Returns [`ErrorKind::Conflict`] if the transaction is closed,
    /// [`ErrorKind::AlreadyExists`] for a duplicate identity or URI, or
    /// [`ErrorKind::Storage`] for other persistence failures.
    pub fn add_media_root(&mut self, root: MediaRoot) -> Result<()> {
        let transaction = self.open_transaction()?;
        transaction
            .execute(
                "INSERT INTO media_roots (id, uri, label, priority, enabled)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    root.id().as_bytes().as_slice(),
                    root.uri(),
                    root.label(),
                    root.priority(),
                    root.is_enabled(),
                ],
            )
            .map_err(mutation_error("persist media root"))?;
        self.pending_roots.push(root);
        Ok(())
    }

    /// Stages an external identifier attachment.
    ///
    /// # Errors
    ///
    /// Returns [`ErrorKind::NotFound`] when the target does not exist,
    /// [`ErrorKind::AlreadyExists`] for an identical attachment,
    /// [`ErrorKind::Unsupported`] for a target kind not yet persisted, or a
    /// transaction/storage error.
    pub fn add_external_identifier(
        &mut self,
        target: ObjectRef,
        identifier: &ExternalIdentifier,
    ) -> Result<()> {
        let (target_kind, target_id) = encode_identifier_target(&target)?;
        let transaction = self.open_transaction()?;
        if !identifier_target_exists(transaction, target_kind, target_id)? {
            return Err(Error::new(
                ErrorKind::NotFound,
                "external identifier target does not exist",
            ));
        }
        transaction
            .execute(
                "INSERT INTO external_identifiers (
                    target_kind, target_id, scheme, value, qualifier
                 ) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    target_kind,
                    target_id.as_slice(),
                    identifier.scheme().as_str(),
                    identifier.value(),
                    identifier.qualifier(),
                ],
            )
            .map(|_| ())
            .map_err(mutation_error("persist external identifier"))
    }

    /// Stages removal of an exact external identifier attachment.
    ///
    /// # Errors
    ///
    /// Returns [`ErrorKind::NotFound`] if the attachment does not exist,
    /// [`ErrorKind::Unsupported`] for a target kind not yet persisted, or a
    /// transaction/storage error.
    pub fn remove_external_identifier(
        &mut self,
        target: ObjectRef,
        identifier: &ExternalIdentifier,
    ) -> Result<()> {
        let (target_kind, target_id) = encode_identifier_target(&target)?;
        let changed = self
            .open_transaction()?
            .execute(
                "DELETE FROM external_identifiers
                 WHERE target_kind = ?1 AND target_id = ?2
                   AND scheme = ?3 AND value = ?4 AND qualifier IS ?5",
                params![
                    target_kind,
                    target_id.as_slice(),
                    identifier.scheme().as_str(),
                    identifier.value(),
                    identifier.qualifier(),
                ],
            )
            .map_err(mutation_error("remove external identifier"))?;
        if changed == 0 {
            return Err(Error::new(
                ErrorKind::NotFound,
                "external identifier attachment does not exist",
            ));
        }
        Ok(())
    }

    /// Appends one ordered value to a metadata property.
    ///
    /// # Errors
    ///
    /// Returns [`ErrorKind::NotFound`] when the target does not exist,
    /// [`ErrorKind::Unsupported`] for activity metadata before activities are
    /// persisted, or a transaction/encoding/storage error.
    pub fn add_metadata_value(
        &mut self,
        target: ObjectRef,
        property: &MetadataProperty,
        value: &MetadataValue,
    ) -> Result<()> {
        let encoded = metadata_codec::encode(value)?;
        let (target_kind, target_id) = encode_metadata_target(&target)?;
        let transaction = self.open_transaction()?;
        ensure_metadata_target_exists(transaction, target_kind, target_id)?;
        let position = next_metadata_position(transaction, target_kind, target_id, property)?;
        insert_metadata_value(
            transaction,
            target_kind,
            target_id,
            property,
            position,
            &encoded,
        )
    }

    /// Replaces all ordered values of a metadata property atomically.
    ///
    /// An empty slice removes all values without treating absence as an error.
    ///
    /// # Errors
    ///
    /// Returns [`ErrorKind::NotFound`] when the target does not exist,
    /// [`ErrorKind::Unsupported`] for activity metadata before activities are
    /// persisted, or a transaction/encoding/storage error.
    pub fn replace_metadata_values(
        &mut self,
        target: ObjectRef,
        property: &MetadataProperty,
        values: &[MetadataValue],
    ) -> Result<()> {
        let encoded = values
            .iter()
            .map(metadata_codec::encode)
            .collect::<Result<Vec<_>>>()?;
        let (target_kind, target_id) = encode_metadata_target(&target)?;
        let transaction = self.open_transaction()?;
        ensure_metadata_target_exists(transaction, target_kind, target_id)?;
        delete_metadata_property(transaction, target_kind, target_id, property)?;
        for (position, value) in encoded.iter().enumerate() {
            let position = i64::try_from(position).map_err(|error| {
                Error::new(
                    ErrorKind::Unsupported,
                    format!("metadata value position cannot be stored: {error}"),
                )
            })?;
            insert_metadata_value(
                transaction,
                target_kind,
                target_id,
                property,
                position,
                value,
            )?;
        }
        Ok(())
    }

    /// Removes all values of one metadata property.
    ///
    /// # Errors
    ///
    /// Returns [`ErrorKind::NotFound`] when the property is absent,
    /// [`ErrorKind::Unsupported`] for an unsupported target kind, or a
    /// transaction/storage error.
    pub fn remove_metadata_property(
        &mut self,
        target: ObjectRef,
        property: &MetadataProperty,
    ) -> Result<()> {
        let (target_kind, target_id) = encode_metadata_target(&target)?;
        let changed =
            delete_metadata_property(self.open_transaction()?, target_kind, target_id, property)?;
        if changed == 0 {
            return Err(Error::new(
                ErrorKind::NotFound,
                "metadata property does not exist on target",
            ));
        }
        Ok(())
    }

    /// Atomically commits all staged mutations.
    ///
    /// # Errors
    ///
    /// Returns [`ErrorKind::Conflict`] if already closed, or
    /// [`ErrorKind::Storage`] if SQLite cannot commit.
    pub fn commit(&mut self) -> Result<()> {
        self.lifecycle.ensure_open()?;
        let transaction = self.take_transaction()?;
        if let Err(error) = transaction.commit() {
            self.lifecycle.mark_rolled_back()?;
            return Err(sqlite_error("commit domain transaction")(error));
        }
        self.lifecycle.mark_committed()?;
        let mut roots = self.project.media_roots().to_vec();
        roots.append(&mut self.pending_roots);
        self.project.set_media_roots(roots);
        Ok(())
    }

    /// Explicitly discards all staged mutations.
    ///
    /// # Errors
    ///
    /// Returns [`ErrorKind::Conflict`] if already closed, or
    /// [`ErrorKind::Storage`] if SQLite cannot roll back.
    pub fn rollback(&mut self) -> Result<()> {
        self.lifecycle.ensure_open()?;
        let transaction = self.take_transaction()?;
        transaction
            .rollback()
            .map_err(sqlite_error("roll back domain transaction"))?;
        self.lifecycle.mark_rolled_back()?;
        self.pending_roots.clear();
        Ok(())
    }

    fn open_transaction(&mut self) -> Result<&Transaction<'project>> {
        self.lifecycle.ensure_open()?;
        self.transaction.as_ref().ok_or_else(|| {
            Error::new(
                ErrorKind::Internal,
                "open transaction has no SQLite transaction",
            )
        })
    }

    fn take_transaction(&mut self) -> Result<Transaction<'project>> {
        self.transaction.take().ok_or_else(|| {
            Error::new(
                ErrorKind::Internal,
                "open transaction has no SQLite transaction",
            )
        })
    }
}

impl ProjectStoreTransaction for SqliteTransaction<'_> {
    fn id(&self) -> TransactionId {
        SqliteTransaction::id(self)
    }

    fn state(&self) -> TransactionState {
        SqliteTransaction::state(self)
    }

    fn import_original(&mut self, import: &OriginalMediaImport) -> Result<()> {
        SqliteTransaction::import_original(self, import)
    }

    fn add_locator(&mut self, locator: &Locator) -> Result<()> {
        SqliteTransaction::add_locator(self, locator)
    }

    fn add_media_root(&mut self, root: MediaRoot) -> Result<()> {
        SqliteTransaction::add_media_root(self, root)
    }

    fn add_external_identifier(
        &mut self,
        target: ObjectRef,
        identifier: &ExternalIdentifier,
    ) -> Result<()> {
        SqliteTransaction::add_external_identifier(self, target, identifier)
    }

    fn remove_external_identifier(
        &mut self,
        target: ObjectRef,
        identifier: &ExternalIdentifier,
    ) -> Result<()> {
        SqliteTransaction::remove_external_identifier(self, target, identifier)
    }

    fn add_metadata_value(
        &mut self,
        target: ObjectRef,
        property: &MetadataProperty,
        value: &MetadataValue,
    ) -> Result<()> {
        SqliteTransaction::add_metadata_value(self, target, property, value)
    }

    fn replace_metadata_values(
        &mut self,
        target: ObjectRef,
        property: &MetadataProperty,
        values: &[MetadataValue],
    ) -> Result<()> {
        SqliteTransaction::replace_metadata_values(self, target, property, values)
    }

    fn remove_metadata_property(
        &mut self,
        target: ObjectRef,
        property: &MetadataProperty,
    ) -> Result<()> {
        SqliteTransaction::remove_metadata_property(self, target, property)
    }

    fn commit(&mut self) -> Result<()> {
        SqliteTransaction::commit(self)
    }

    fn rollback(&mut self) -> Result<()> {
        SqliteTransaction::rollback(self)
    }
}

fn encode_structure_kind(value: ContentStructureKind) -> Result<i64> {
    match value {
        ContentStructureKind::SingleResource => Ok(0),
        ContentStructureKind::ImageSequence => Ok(1),
        ContentStructureKind::OrderedParts => Ok(2),
        ContentStructureKind::Package => Ok(3),
        _ => Err(Error::new(
            ErrorKind::Unsupported,
            "content structure is not supported by this schema",
        )),
    }
}

fn encode_availability(value: LocatorAvailability) -> Result<i64> {
    match value {
        LocatorAvailability::Unknown => Ok(0),
        LocatorAvailability::Online => Ok(1),
        LocatorAvailability::Offline => Ok(2),
        _ => Err(Error::new(
            ErrorKind::Unsupported,
            "locator availability is not supported by this schema",
        )),
    }
}

fn persist_resource(transaction: &Transaction<'_>, resource: &Resource) -> Result<()> {
    let size = resource
        .file_facts()
        .map(|facts| {
            i64::try_from(facts.size_bytes()).map_err(|error| {
                Error::new(
                    ErrorKind::Unsupported,
                    format!("media resource is too large for SQLite storage: {error}"),
                )
            })
        })
        .transpose()?;
    let modified_at = resource
        .file_facts()
        .and_then(postproject_core::FileFacts::modified_at)
        .map(postproject_core::Timestamp::as_unix_micros);
    transaction
        .execute(
            "INSERT INTO resources (id, file_size_bytes, modified_at_micros)
             VALUES (?1, ?2, ?3)",
            params![resource.id().as_bytes().as_slice(), size, modified_at],
        )
        .map_err(mutation_error("persist resource"))?;
    for fingerprint in resource.fingerprints() {
        transaction
            .execute(
                "INSERT INTO resource_fingerprints (
                    resource_id, algorithm, algorithm_version, value
                 ) VALUES (?1, ?2, ?3, ?4)",
                params![
                    resource.id().as_bytes().as_slice(),
                    fingerprint.algorithm(),
                    fingerprint.version(),
                    fingerprint.value(),
                ],
            )
            .map_err(mutation_error("persist resource fingerprint"))?;
    }
    Ok(())
}

fn persist_content_structure(
    transaction: &Transaction<'_>,
    representation_id: postproject_core::RepresentationId,
    structure: &ContentStructure,
) -> Result<()> {
    let resource_ids = structure.resource_ids();
    for (position, resource_id) in resource_ids.iter().enumerate() {
        let member = structure
            .members()
            .and_then(|members| members.get(position));
        transaction
            .execute(
                "INSERT INTO representation_resources (
                    representation_id, resource_id, position, role, required
                 ) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    representation_id.as_bytes().as_slice(),
                    resource_id.as_bytes().as_slice(),
                    i64::try_from(position).map_err(|error| Error::new(
                        ErrorKind::Unsupported,
                        format!("content position is too large for SQLite: {error}"),
                    ))?,
                    member.map(|item| item.role().as_str()),
                    member.is_none_or(postproject_core::ResourceMember::is_required),
                ],
            )
            .map_err(mutation_error("persist representation resource"))?;
    }

    if let Some(sequence) = structure.image_sequence_descriptor() {
        let frames = sequence.frames();
        let pattern = sequence.pattern();
        let rate = sequence.rate();
        transaction
            .execute(
                "INSERT INTO image_sequences (
                    representation_id, resource_id, prefix, suffix, padding,
                    start_frame, end_frame, frame_step, rate_numerator, rate_denominator
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                params![
                    representation_id.as_bytes().as_slice(),
                    sequence.resource_id().as_bytes().as_slice(),
                    pattern.prefix(),
                    pattern.suffix(),
                    pattern.padding(),
                    frames.start(),
                    frames.end(),
                    frames.step(),
                    rate.numerator(),
                    rate.denominator(),
                ],
            )
            .map_err(mutation_error("persist image sequence"))?;
        for frame in sequence.known_missing_frames() {
            transaction
                .execute(
                    "INSERT INTO image_sequence_missing_frames (representation_id, frame)
                     VALUES (?1, ?2)",
                    params![representation_id.as_bytes().as_slice(), frame],
                )
                .map_err(mutation_error("persist missing sequence frame"))?;
        }
    }
    Ok(())
}

fn persist_locator(transaction: &Transaction<'_>, locator: &Locator) -> Result<()> {
    let availability = encode_availability(locator.availability())?;
    transaction
        .execute(
            "INSERT INTO locators (
                id, resource_id, uri, last_seen_micros, availability
             ) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                locator.id().as_bytes().as_slice(),
                locator.resource_id().as_bytes().as_slice(),
                locator.uri(),
                locator
                    .last_seen()
                    .map(postproject_core::Timestamp::as_unix_micros),
                availability,
            ],
        )
        .map(|_| ())
        .map_err(mutation_error("persist resource locator"))
}

fn identifier_target_exists(
    transaction: &Transaction<'_>,
    target_kind: i64,
    target_id: &[u8; 16],
) -> Result<bool> {
    let table = match target_kind {
        1 => "assets",
        2 => "representations",
        3 => "resources",
        _ => {
            return Err(Error::new(
                ErrorKind::Unsupported,
                "external identifier target kind is not supported by this schema",
            ));
        }
    };
    transaction
        .query_row(
            &format!("SELECT EXISTS(SELECT 1 FROM {table} WHERE id = ?1)"),
            [target_id.as_slice()],
            |row| row.get(0),
        )
        .map_err(sqlite_error("check external identifier target"))
}

fn ensure_metadata_target_exists(
    transaction: &Transaction<'_>,
    target_kind: i64,
    target_id: &[u8; 16],
) -> Result<()> {
    let table = match target_kind {
        0 => "projects",
        1 => "assets",
        2 => "representations",
        3 => "resources",
        4 => {
            return Err(Error::new(
                ErrorKind::Unsupported,
                "activity metadata requires activity persistence",
            ));
        }
        _ => {
            return Err(Error::new(
                ErrorKind::Unsupported,
                "metadata target kind is not supported by this schema",
            ));
        }
    };
    let exists = transaction
        .query_row(
            &format!("SELECT EXISTS(SELECT 1 FROM {table} WHERE id = ?1)"),
            [target_id.as_slice()],
            |row| row.get::<_, bool>(0),
        )
        .map_err(sqlite_error("check metadata target"))?;
    if !exists {
        return Err(Error::new(
            ErrorKind::NotFound,
            "metadata target does not exist",
        ));
    }
    Ok(())
}

fn next_metadata_position(
    transaction: &Transaction<'_>,
    target_kind: i64,
    target_id: &[u8; 16],
    property: &MetadataProperty,
) -> Result<i64> {
    transaction
        .query_row(
            "SELECT COALESCE(MAX(position) + 1, 0)
             FROM metadata_assertions
             WHERE target_kind = ?1 AND target_id = ?2
               AND vocabulary = ?3 AND property = ?4",
            params![
                target_kind,
                target_id.as_slice(),
                property.vocabulary().as_str(),
                property.property().as_str(),
            ],
            |row| row.get(0),
        )
        .map_err(sqlite_error("choose metadata value position"))
}

fn insert_metadata_value(
    transaction: &Transaction<'_>,
    target_kind: i64,
    target_id: &[u8; 16],
    property: &MetadataProperty,
    position: i64,
    encoded: &[u8],
) -> Result<()> {
    transaction
        .execute(
            "INSERT INTO metadata_assertions (
                target_kind, target_id, vocabulary, property, position, encoded_value
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                target_kind,
                target_id.as_slice(),
                property.vocabulary().as_str(),
                property.property().as_str(),
                position,
                encoded,
            ],
        )
        .map(|_| ())
        .map_err(mutation_error("persist metadata value"))
}

fn delete_metadata_property(
    transaction: &Transaction<'_>,
    target_kind: i64,
    target_id: &[u8; 16],
    property: &MetadataProperty,
) -> Result<usize> {
    transaction
        .execute(
            "DELETE FROM metadata_assertions
             WHERE target_kind = ?1 AND target_id = ?2
               AND vocabulary = ?3 AND property = ?4",
            params![
                target_kind,
                target_id.as_slice(),
                property.vocabulary().as_str(),
                property.property().as_str(),
            ],
        )
        .map_err(mutation_error("remove metadata property"))
}

fn mutation_error(context: &'static str) -> impl FnOnce(rusqlite::Error) -> Error {
    move |error| {
        let kind = if error.sqlite_error_code() == Some(ErrorCode::ConstraintViolation) {
            ErrorKind::AlreadyExists
        } else {
            ErrorKind::Storage
        };
        Error::new(kind, format!("{context}: {error}"))
    }
}
