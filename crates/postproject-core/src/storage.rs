//! Domain-shaped contracts implemented by persistence backends.

use crate::{
    Activity, ArtifactEvaluation, ArtifactEvaluationLimits, ArtifactReproducibilityReport, Asset,
    AssetId, Dependency, DependencySet, ExternalIdentifier, IdentifierScheme, Locator, MediaRoot,
    MetadataAssertion, MetadataMatch, MetadataProperty, MetadataValue, ObjectRef,
    OriginalMediaImport, Production, Representation, RepresentationFingerprint, RepresentationId,
    RepresentationImport, Resource, ResourceFingerprint, ResourceId, Result, Revision,
    RevisionContext, RevisionEvent, RevisionId, TransactionId, TransactionState,
};

/// Read operations required from a production persistence backend.
///
/// The contract returns domain values and deliberately contains no generic CRUD,
/// query language, connection, or database-row concepts.
pub trait ProductionRead {
    /// Returns the loaded production metadata and configured media roots.
    fn production(&self) -> &Production;

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

    /// Loads the complete dependency observation for a representation.
    ///
    /// `None` means that no dependency set has been recorded. An empty set is
    /// returned as `Some` and is distinct from missing knowledge.
    ///
    /// # Errors
    ///
    /// Returns a domain error when the representation is absent or persisted
    /// dependency data cannot be decoded safely.
    fn dependency_set(&self, representation_id: RepresentationId) -> Result<Option<DependencySet>>;

    /// Evaluates whether an activity-produced representation still reflects
    /// its recorded inputs and output snapshot.
    ///
    /// This operation reads production knowledge only and never resolves or
    /// accesses media files.
    ///
    /// # Errors
    ///
    /// Returns a domain error when the target is absent, bounds are invalid,
    /// or stored provenance cannot be decoded safely.
    fn evaluate_artifact(
        &self,
        representation_id: RepresentationId,
        limits: ArtifactEvaluationLimits,
    ) -> Result<ArtifactEvaluation>;

    /// Reports whether recorded production knowledge can reproduce an artifact.
    ///
    /// # Errors
    ///
    /// Returns a domain error when the target is absent or stored provenance
    /// cannot be decoded safely.
    fn artifact_reproducibility(
        &self,
        representation_id: RepresentationId,
    ) -> Result<ArtifactReproducibilityReport>;

    /// Returns the newest durable revision, or `None` for an empty journal.
    ///
    /// # Errors
    ///
    /// Returns a storage-domain error when persisted revision data is invalid.
    fn latest_revision(&self) -> Result<Option<Revision>>;

    /// Returns revisions after `sequence` in ascending order, capped by `limit`.
    ///
    /// # Errors
    ///
    /// Returns a domain error when `limit` is zero or excessive, or when
    /// persisted revision data is invalid.
    fn changes_since(&self, sequence: u64, limit: u32) -> Result<Vec<Revision>>;

    /// Loads the semantic events for one revision in stable position order.
    ///
    /// # Errors
    ///
    /// Returns a domain error when the revision is absent or persisted event
    /// data is invalid.
    fn events_for_revision(&self, revision_id: RevisionId) -> Result<Vec<RevisionEvent>>;
}

/// Transactional mutation operations required from a persistence backend.
pub trait ProductionStoreTransaction {
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

    /// Stages a representation and its newly imported resources on an existing asset.
    ///
    /// # Errors
    ///
    /// Returns a domain error when the transaction is closed, the owning asset
    /// does not exist, or persistence rejects the aggregate.
    fn add_representation(&mut self, import: &RepresentationImport) -> Result<()>;

    /// Stages an explicitly confirmed resource locator.
    ///
    /// # Errors
    ///
    /// Returns a domain error when the transaction is closed or persistence
    /// rejects the locator.
    fn add_locator(&mut self, locator: &Locator) -> Result<()>;

    /// Stages retirement of one superseded resource locator.
    ///
    /// # Errors
    ///
    /// Returns a domain error when the transaction is closed, the locator does
    /// not exist, or persistence fails.
    fn retire_locator(&mut self, locator_id: crate::LocatorId) -> Result<()>;

    /// Stages a configured resolver search root.
    ///
    /// # Errors
    ///
    /// Returns a domain error when the transaction is closed or persistence
    /// rejects the root.
    fn add_media_root(&mut self, root: MediaRoot) -> Result<()>;

    /// Enables or disables a configured resolver search root.
    ///
    /// Setting the existing state is an idempotent no-op.
    ///
    /// # Errors
    ///
    /// Returns a domain error when the transaction is closed, the root does not
    /// exist, or persistence fails.
    fn set_media_root_enabled(&mut self, root_id: crate::MediaRootId, enabled: bool) -> Result<()>;

    /// Stages removal of a configured resolver search root.
    ///
    /// # Errors
    ///
    /// Returns a domain error when the transaction is closed, the root does not
    /// exist, or persistence fails.
    fn remove_media_root(&mut self, root_id: crate::MediaRootId) -> Result<()>;

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

    /// Replaces one representation's complete ordered dependency observation.
    ///
    /// Returns `true` when state changed and `false` for an identical current
    /// observation. An empty slice explicitly records a known empty set.
    ///
    /// # Errors
    ///
    /// Returns a domain error when the transaction is closed, a referenced
    /// object is absent or inconsistent, or persistence fails.
    fn record_dependency_set(
        &mut self,
        representation_id: RepresentationId,
        dependencies: &[Dependency],
    ) -> Result<bool>;

    /// Records a resource fingerprint as the current observation in its domain.
    ///
    /// Returns `true` when state changed and `false` for an identical no-op.
    ///
    /// # Errors
    ///
    /// Returns a domain error when the transaction is closed, the resource is
    /// absent, or persistence fails.
    fn record_resource_fingerprint(
        &mut self,
        resource_id: ResourceId,
        fingerprint: &ResourceFingerprint,
    ) -> Result<bool>;

    /// Records a representation fingerprint as the current observation.
    ///
    /// Returns `true` when state changed and `false` for an identical no-op.
    /// A changed observation clears that representation's recomputation marker.
    ///
    /// # Errors
    ///
    /// Returns a domain error when the transaction is closed, the
    /// representation is absent, or persistence fails.
    fn record_representation_fingerprint(
        &mut self,
        representation_id: RepresentationId,
        fingerprint: &RepresentationFingerprint,
    ) -> Result<bool>;

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

/// A production persistence backend with explicit domain transactions.
pub trait ProductionStore: ProductionRead {
    /// Backend-specific transaction implementation borrowing this store.
    type Transaction<'production>: ProductionStoreTransaction
    where
        Self: 'production;

    /// Begins a transaction for domain mutations.
    ///
    /// # Errors
    ///
    /// Returns a storage-domain error when a transaction cannot be started.
    fn begin_transaction(&mut self) -> Result<Self::Transaction<'_>>;
}
