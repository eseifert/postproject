//! Domain-shaped contracts implemented by persistence backends.

use crate::{
    Asset, AssetId, ExternalIdentifier, IdentifierScheme, Location, MediaRoot, ObjectRef,
    OriginalMediaImport, Project, Representation, RepresentationId, Result, TransactionId,
    TransactionState,
};

/// Read operations required from a project persistence backend.
///
/// The contract returns domain values and deliberately contains no generic CRUD,
/// query language, connection, or database-row concepts.
pub trait ProjectRead {
    /// Returns the loaded project metadata and configured media roots.
    fn project(&self) -> &Project;

    /// Loads all assets in deterministic order.
    ///
    /// # Errors
    ///
    /// Returns a storage-domain error when persisted data cannot be read or
    /// decoded safely.
    fn assets(&self) -> Result<Vec<Asset>>;

    /// Loads every representation belonging to an asset in deterministic order.
    ///
    /// # Errors
    ///
    /// Returns a storage-domain error when persisted data cannot be read or
    /// decoded safely.
    fn representations(&self, asset_id: AssetId) -> Result<Vec<Representation>>;

    /// Loads every known location belonging to a representation.
    ///
    /// # Errors
    ///
    /// Returns a storage-domain error when persisted data cannot be read or
    /// decoded safely.
    fn locations(&self, representation_id: RepresentationId) -> Result<Vec<Location>>;

    /// Loads external identifiers attached to `target` in deterministic order.
    ///
    /// # Errors
    ///
    /// Returns a storage-domain error when persisted data cannot be read or
    /// decoded safely, or when the target kind is not supported.
    fn external_identifiers(&self, target: ObjectRef) -> Result<Vec<ExternalIdentifier>>;

    /// Finds objects carrying the exact external scheme and value.
    ///
    /// # Errors
    ///
    /// Returns a domain error when the lookup value is invalid or persisted
    /// data cannot be decoded safely.
    fn find_by_external_identifier(
        &self,
        scheme: &IdentifierScheme,
        value: &str,
    ) -> Result<Vec<ObjectRef>>;
}

/// Transactional mutation operations required from a persistence backend.
pub trait ProjectStoreTransaction {
    /// Returns this transaction's stable identity.
    fn id(&self) -> TransactionId;

    /// Returns the current lifecycle state.
    fn state(&self) -> TransactionState;

    /// Stages one prepared original-media aggregate atomically.
    ///
    /// # Errors
    ///
    /// Returns a domain error when the transaction is closed or persistence
    /// rejects the aggregate.
    fn import_original(&mut self, import: &OriginalMediaImport) -> Result<()>;

    /// Stages an explicitly confirmed representation location.
    ///
    /// # Errors
    ///
    /// Returns a domain error when the transaction is closed or persistence
    /// rejects the location.
    fn add_location(&mut self, location: &Location) -> Result<()>;

    /// Stages a configured resolver search root.
    ///
    /// # Errors
    ///
    /// Returns a domain error when the transaction is closed or persistence
    /// rejects the root.
    fn add_media_root(&mut self, root: MediaRoot) -> Result<()>;

    /// Stages an external identifier attachment.
    ///
    /// # Errors
    ///
    /// Returns a domain error when the transaction is closed, the target does
    /// not exist, the attachment already exists, or persistence fails.
    fn add_external_identifier(
        &mut self,
        target: ObjectRef,
        identifier: &ExternalIdentifier,
    ) -> Result<()>;

    /// Stages removal of one exact external identifier attachment.
    ///
    /// # Errors
    ///
    /// Returns a domain error when the transaction is closed, the attachment
    /// does not exist, the target kind is unsupported, or persistence fails.
    fn remove_external_identifier(
        &mut self,
        target: ObjectRef,
        identifier: &ExternalIdentifier,
    ) -> Result<()>;

    /// Atomically makes every staged mutation durable.
    ///
    /// # Errors
    ///
    /// Returns a domain error when the transaction is closed or commit fails.
    fn commit(&mut self) -> Result<()>;

    /// Explicitly discards every staged mutation.
    ///
    /// # Errors
    ///
    /// Returns a domain error when the transaction is closed or rollback fails.
    fn rollback(&mut self) -> Result<()>;
}

/// A project persistence backend with explicit domain transactions.
pub trait ProjectStore: ProjectRead {
    /// Backend-specific transaction implementation borrowing this store.
    type Transaction<'project>: ProjectStoreTransaction
    where
        Self: 'project;

    /// Begins a transaction for domain mutations.
    ///
    /// # Errors
    ///
    /// Returns a storage-domain error when a transaction cannot be started.
    fn begin_transaction(&mut self) -> Result<Self::Transaction<'_>>;
}
