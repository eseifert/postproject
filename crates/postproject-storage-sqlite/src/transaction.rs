//! Explicit SQLite-backed domain transactions.

use postproject_core::{
    Error, ErrorKind, LocationAvailability, MediaRoot, OriginalMediaImport, Project, Result,
    TransactionId, TransactionLifecycle, TransactionState,
};
use rusqlite::{Connection, ErrorCode, Transaction, TransactionBehavior, params};

use crate::sqlite_error;

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
        let location = import.location();
        let size = representation
            .file_facts()
            .map(|facts| {
                i64::try_from(facts.size_bytes()).map_err(|error| {
                    Error::new(
                        ErrorKind::Unsupported,
                        format!("media file is too large for SQLite storage: {error}"),
                    )
                })
            })
            .transpose()?;
        let modified_at = representation
            .file_facts()
            .and_then(postproject_core::FileFacts::modified_at)
            .map(postproject_core::Timestamp::as_unix_micros);
        let availability = encode_availability(location.availability())?;

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
                    id, asset_id, kind, file_size_bytes, modified_at_micros
                 ) VALUES (?1, ?2, 0, ?3, ?4)",
                params![
                    representation.id().as_bytes().as_slice(),
                    asset.id().as_bytes().as_slice(),
                    size,
                    modified_at,
                ],
            )
            .map_err(mutation_error("persist original representation"))?;
        if let Some(fingerprint) = representation.fingerprint() {
            transaction
                .execute(
                    "INSERT INTO fingerprints (
                        representation_id, algorithm, algorithm_version, value
                     ) VALUES (?1, ?2, ?3, ?4)",
                    params![
                        representation.id().as_bytes().as_slice(),
                        fingerprint.algorithm(),
                        fingerprint.version(),
                        fingerprint.value(),
                    ],
                )
                .map_err(mutation_error("persist original fingerprint"))?;
        }
        persist_location(transaction, location, availability)?;
        Ok(())
    }

    /// Stages an additional confirmed location for a representation.
    ///
    /// # Errors
    ///
    /// Returns [`ErrorKind::Conflict`] if the transaction is closed,
    /// [`ErrorKind::AlreadyExists`] for a duplicate location or missing owning
    /// representation, or [`ErrorKind::Storage`] for other persistence failures.
    pub fn add_location(&mut self, location: &postproject_core::Location) -> Result<()> {
        let availability = encode_availability(location.availability())?;
        persist_location(self.open_transaction()?, location, availability)
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

fn encode_availability(value: LocationAvailability) -> Result<i64> {
    match value {
        LocationAvailability::Unknown => Ok(0),
        LocationAvailability::Online => Ok(1),
        LocationAvailability::Offline => Ok(2),
        _ => Err(Error::new(
            ErrorKind::Unsupported,
            "location availability is not supported by this schema",
        )),
    }
}

fn persist_location(
    transaction: &Transaction<'_>,
    location: &postproject_core::Location,
    availability: i64,
) -> Result<()> {
    transaction
        .execute(
            "INSERT INTO locations (
                id, representation_id, uri, last_seen_micros, availability
             ) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                location.id().as_bytes().as_slice(),
                location.representation_id().as_bytes().as_slice(),
                location.uri(),
                location
                    .last_seen()
                    .map(postproject_core::Timestamp::as_unix_micros),
                availability,
            ],
        )
        .map(|_| ())
        .map_err(mutation_error("persist representation location"))
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
