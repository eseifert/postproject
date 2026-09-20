//! Domain-shaped contracts implemented by persistence backends.

use crate::{
    Activity, Asset, AssetId, ExternalIdentifier, IdentifierScheme, Locator, MediaRoot,
    MetadataAssertion, MetadataMatch, MetadataProperty, MetadataValue, ObjectRef,
    OriginalMediaImport, Project, Representation, RepresentationId, Resource, ResourceId, Result,
    RevisionContext, TransactionId, TransactionState,
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

    /// Loads resources used by a representation in structural order.
    ///
    /// # Errors
    ///
    /// Returns a storage-domain error when persisted data cannot be read or
    /// decoded safely.
    fn resources(&self, representation_id: RepresentationId) -> Result<Vec<Resource>>;

    /// Loads every known locator belonging to a resource.
    ///
    /// # Errors
    ///
    /// Returns a storage-domain error when persisted data cannot be read or
    /// decoded safely.
    fn locators(&self, resource_id: ResourceId) -> Result<Vec<Locator>>;

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

    /// Loads all metadata assertions attached to `target` in deterministic order.
    ///
    /// # Errors
    ///
    /// Returns a domain error when the target kind is unsupported or persisted
    /// data cannot be decoded safely.
    fn metadata(&self, target: ObjectRef) -> Result<Vec<MetadataAssertion>>;

    /// Loads every ordered value for one property on `target`.
    ///
    /// # Errors
    ///
    /// Returns a domain error when the target kind is unsupported or persisted
    /// data cannot be decoded safely.
    fn metadata_values(
        &self,
        target: ObjectRef,
        property: &MetadataProperty,
    ) -> Result<Vec<MetadataValue>>;

    /// Finds every object carrying `property`, preserving value repetition.
    ///
    /// # Errors
    ///
    /// Returns a domain error when persisted data cannot be decoded safely.
    fn query_by_metadata_property(&self, property: &MetadataProperty)
    -> Result<Vec<MetadataMatch>>;

    /// Loads all production activities in deterministic identity order.
    ///
    /// # Errors
    ///
    /// Returns a storage-domain error when persisted activity data cannot be
    /// read or decoded safely.
    fn activities(&self) -> Result<Vec<Activity>>;

    /// Loads activities that produce `representation_id`.
    ///
    /// # Errors
    ///
    /// Returns a domain error when the representation is absent or persisted
    /// activity data cannot be read safely.
    fn activities_producing(&self, representation_id: RepresentationId) -> Result<Vec<Activity>>;

    /// Loads activities that consume `representation_id`.
    ///
    /// # Errors
    ///
    /// Returns a domain error when the representation is absent or persisted
    /// activity data cannot be read safely.
    fn activities_consuming(&self, representation_id: RepresentationId) -> Result<Vec<Activity>>;

    /// Returns every transitive provenance ancestor of `representation_id`.
    ///
    /// # Errors
    ///
    /// Returns a domain error when the representation is absent or persisted
    /// provenance cannot be traversed safely.
    fn ancestors(&self, representation_id: RepresentationId) -> Result<Vec<RepresentationId>>;

    /// Returns every transitive provenance descendant of `representation_id`.
    ///
    /// # Errors
    ///
    /// Returns a domain error when the representation is absent or persisted
    /// provenance cannot be traversed safely.
    fn descendants(&self, representation_id: RepresentationId) -> Result<Vec<RepresentationId>>;
}

/// Transactional mutation operations required from a persistence backend.
pub trait ProjectStoreTransaction {
    /// Returns this transaction's stable identity.
    fn id(&self) -> TransactionId;

    /// Returns the current lifecycle state.
    fn state(&self) -> TransactionState;

    /// Sets the origin and message for the revision created on commit.
    ///
    /// # Errors
    ///
    /// Returns a domain error when the transaction is already closed.
    fn set_revision_context(&mut self, context: RevisionContext) -> Result<()>;

    /// Stages one prepared original-media aggregate atomically.
    ///
    /// # Errors
    ///
    /// Returns a domain error when the transaction is closed or persistence
    /// rejects the aggregate.
    fn import_original(&mut self, import: &OriginalMediaImport) -> Result<()>;

    /// Stages an explicitly confirmed resource locator.
    ///
    /// # Errors
    ///
    /// Returns a domain error when the transaction is closed or persistence
    /// rejects the locator.
    fn add_locator(&mut self, locator: &Locator) -> Result<()>;

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

    /// Appends one value to an object's metadata property.
    ///
    /// # Errors
    ///
    /// Returns a domain error when the transaction is closed, the target does
    /// not exist or is unsupported, encoding fails, or persistence fails.
    fn add_metadata_value(
        &mut self,
        target: ObjectRef,
        property: &MetadataProperty,
        value: &MetadataValue,
    ) -> Result<()>;

    /// Atomically replaces all values of one metadata property.
    ///
    /// An empty value slice removes the property.
    ///
    /// # Errors
    ///
    /// Returns a domain error when the transaction is closed, the target does
    /// not exist or is unsupported, encoding fails, or persistence fails.
    fn replace_metadata_values(
        &mut self,
        target: ObjectRef,
        property: &MetadataProperty,
        values: &[MetadataValue],
    ) -> Result<()>;

    /// Removes all values of one metadata property.
    ///
    /// # Errors
    ///
    /// Returns a domain error when the transaction is closed, the property is
    /// absent, the target kind is unsupported, or persistence fails.
    fn remove_metadata_property(
        &mut self,
        target: ObjectRef,
        property: &MetadataProperty,
    ) -> Result<()>;

    /// Stages a complete production activity with its input and output edges.
    ///
    /// # Errors
    ///
    /// Returns a domain error when the transaction is closed, a referenced
    /// representation is absent, the activity already exists, its edges would
    /// create a provenance cycle, or persistence fails.
    fn create_activity(&mut self, activity: &Activity) -> Result<()>;

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
