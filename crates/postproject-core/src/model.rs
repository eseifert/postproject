//! Core media identity and location value types.

use std::{
    collections::BTreeSet,
    time::{SystemTime, UNIX_EPOCH},
};

use crate::{
    AssetId, ContentStructure, Error, ErrorKind, Locator, MediaRootId, ProjectId,
    RepresentationFingerprint, RepresentationId, Resource, Result, uri::normalize_uri,
};

/// A UTC instant represented as microseconds since the Unix epoch.
///
/// The representation is independent of SQLite and has sufficient precision for
/// filesystem and domain bookkeeping without exposing a third-party time type.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Timestamp(i64);

impl Timestamp {
    /// Creates a timestamp from signed Unix microseconds.
    #[must_use]
    pub const fn from_unix_micros(micros: i64) -> Self {
        Self(micros)
    }

    /// Returns the signed Unix-microsecond representation.
    #[must_use]
    pub const fn as_unix_micros(self) -> i64 {
        self.0
    }

    /// Reads the current system time.
    ///
    /// # Errors
    ///
    /// Returns [`ErrorKind::Internal`] if the platform clock predates the Unix
    /// epoch or cannot be represented in signed microseconds.
    pub fn now() -> Result<Self> {
        let duration = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|error| {
                Error::new(
                    ErrorKind::Internal,
                    format!("system clock is before the Unix epoch: {error}"),
                )
            })?;
        let micros = i64::try_from(duration.as_micros()).map_err(|error| {
            Error::new(
                ErrorKind::Internal,
                format!("system clock is outside the supported range: {error}"),
            )
        })?;
        Ok(Self(micros))
    }
}

/// A persistent container for production state.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Project {
    id: ProjectId,
    schema_version: u32,
    created_at: Timestamp,
    display_name: Option<String>,
    media_roots: Vec<MediaRoot>,
}

impl Project {
    /// Creates an in-memory project value.
    #[must_use]
    pub fn new(
        id: ProjectId,
        schema_version: u32,
        created_at: Timestamp,
        display_name: Option<String>,
    ) -> Self {
        Self {
            id,
            schema_version,
            created_at,
            display_name,
            media_roots: Vec::new(),
        }
    }

    /// Returns the project's stable identity.
    #[must_use]
    pub const fn id(&self) -> ProjectId {
        self.id
    }

    /// Returns the persistence schema version used to load this project.
    #[must_use]
    pub const fn schema_version(&self) -> u32 {
        self.schema_version
    }

    /// Returns when the project was created.
    #[must_use]
    pub const fn created_at(&self) -> Timestamp {
        self.created_at
    }

    /// Returns the optional user-facing name.
    #[must_use]
    pub fn display_name(&self) -> Option<&str> {
        self.display_name.as_deref()
    }

    /// Returns configured media roots in resolver priority order.
    #[must_use]
    pub fn media_roots(&self) -> &[MediaRoot] {
        &self.media_roots
    }

    /// Replaces media roots after sorting by priority and stable identity.
    pub fn set_media_roots(&mut self, mut roots: Vec<MediaRoot>) {
        roots.sort_by_key(|root| (root.priority(), root.id()));
        self.media_roots = roots;
    }
}

/// Logical identity for one piece of production media.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Asset {
    id: AssetId,
    created_at: Timestamp,
    display_name: Option<String>,
    import_source: Option<String>,
}

impl Asset {
    /// Creates an asset value. Paths belong to locations, not assets.
    #[must_use]
    pub fn new(
        id: AssetId,
        created_at: Timestamp,
        display_name: Option<String>,
        import_source: Option<String>,
    ) -> Self {
        Self {
            id,
            created_at,
            display_name,
            import_source,
        }
    }

    /// Returns the asset's stable identity.
    #[must_use]
    pub const fn id(&self) -> AssetId {
        self.id
    }

    /// Returns when the asset was created.
    #[must_use]
    pub const fn created_at(&self) -> Timestamp {
        self.created_at
    }

    /// Returns the optional user-facing name.
    #[must_use]
    pub fn display_name(&self) -> Option<&str> {
        self.display_name.as_deref()
    }

    /// Returns optional application-supplied import provenance.
    #[must_use]
    pub fn import_source(&self) -> Option<&str> {
        self.import_source.as_deref()
    }
}

/// The semantic role of an asset representation.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum RepresentationKind {
    /// Source media as imported.
    Original,
    /// A lower-cost representation intended for interactive work.
    Proxy,
    /// A representation optimized for a particular workflow.
    Optimized,
    /// Media derived from another production operation.
    Derived,
}

/// Cheap filesystem facts stored with a representation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FileFacts {
    size_bytes: u64,
    modified_at: Option<Timestamp>,
}

impl FileFacts {
    /// Creates file facts from a byte size and optional modification time.
    #[must_use]
    pub const fn new(size_bytes: u64, modified_at: Option<Timestamp>) -> Self {
        Self {
            size_bytes,
            modified_at,
        }
    }

    /// Returns the observed file size.
    #[must_use]
    pub const fn size_bytes(self) -> u64 {
        self.size_bytes
    }

    /// Returns the observed modification time, when available.
    #[must_use]
    pub const fn modified_at(self) -> Option<Timestamp> {
        self.modified_at
    }
}

/// Versioned, deterministic identity evidence derived from file contents.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Fingerprint {
    algorithm: String,
    version: u16,
    value: Vec<u8>,
}

impl Fingerprint {
    /// Creates a fingerprint after validating its extensible algorithm label and value.
    ///
    /// # Errors
    ///
    /// Returns [`ErrorKind::InvalidArgument`] when the algorithm label is empty,
    /// too long, contains unsupported bytes, or when `value` is empty.
    pub fn new(algorithm: impl Into<String>, version: u16, value: Vec<u8>) -> Result<Self> {
        let algorithm = algorithm.into();
        if algorithm.is_empty()
            || algorithm.len() > 64
            || !algorithm
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
        {
            return Err(Error::new(
                ErrorKind::InvalidArgument,
                "fingerprint algorithm must be 1-64 ASCII letters, digits, '-' or '_'",
            ));
        }
        if value.is_empty() {
            return Err(Error::new(
                ErrorKind::InvalidArgument,
                "fingerprint value must not be empty",
            ));
        }
        Ok(Self {
            algorithm,
            version,
            value,
        })
    }

    /// Returns the algorithm identifier.
    #[must_use]
    pub fn algorithm(&self) -> &str {
        &self.algorithm
    }

    /// Returns the algorithm format version.
    #[must_use]
    pub const fn version(&self) -> u16 {
        self.version
    }

    /// Returns the opaque fingerprint bytes.
    #[must_use]
    pub fn value(&self) -> &[u8] {
        &self.value
    }
}

/// One encoded or derived form of an asset.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Representation {
    id: RepresentationId,
    asset_id: AssetId,
    kind: RepresentationKind,
    content_structure: ContentStructure,
    fingerprints: Vec<RepresentationFingerprint>,
}

impl Representation {
    /// Creates a representation value.
    #[must_use]
    pub fn new(
        id: RepresentationId,
        asset_id: AssetId,
        kind: RepresentationKind,
        content_structure: ContentStructure,
        fingerprints: Vec<RepresentationFingerprint>,
    ) -> Self {
        Self {
            id,
            asset_id,
            kind,
            content_structure,
            fingerprints,
        }
    }

    /// Returns the representation's stable identity.
    #[must_use]
    pub const fn id(&self) -> RepresentationId {
        self.id
    }

    /// Returns the owning asset identity.
    #[must_use]
    pub const fn asset_id(&self) -> AssetId {
        self.asset_id
    }

    /// Returns the representation's semantic role.
    #[must_use]
    pub const fn kind(&self) -> RepresentationKind {
        self.kind
    }

    /// Returns how storage resources realize this representation.
    #[must_use]
    pub const fn content_structure(&self) -> &ContentStructure {
        &self.content_structure
    }

    /// Returns structure-aware identity evidence for this representation.
    #[must_use]
    pub fn fingerprints(&self) -> &[RepresentationFingerprint] {
        &self.fingerprints
    }
}

/// A validated aggregate representing an imported original media file.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OriginalMediaImport {
    asset: Asset,
    representation: Representation,
    resources: Vec<Resource>,
    locators: Vec<Locator>,
}

impl OriginalMediaImport {
    /// Creates an import aggregate whose ownership relationships are consistent.
    ///
    /// # Errors
    ///
    /// Returns [`ErrorKind::InvalidArgument`] if ownership is inconsistent, a
    /// referenced resource is absent or duplicated, an extra resource is
    /// supplied, or any resource lacks a locator.
    pub fn new(
        asset: Asset,
        representation: Representation,
        resources: Vec<Resource>,
        locators: Vec<Locator>,
    ) -> Result<Self> {
        if representation.asset_id() != asset.id() {
            return Err(Error::new(
                ErrorKind::InvalidArgument,
                "import representation does not belong to its asset",
            ));
        }
        if representation.kind() != RepresentationKind::Original {
            return Err(Error::new(
                ErrorKind::InvalidArgument,
                "initial import representation must be original media",
            ));
        }
        let expected: BTreeSet<_> = representation
            .content_structure()
            .resource_ids()
            .into_iter()
            .collect();
        let supplied: BTreeSet<_> = resources.iter().map(Resource::id).collect();
        if expected != supplied || supplied.len() != resources.len() {
            return Err(Error::new(
                ErrorKind::InvalidArgument,
                "import resources do not exactly match the content structure",
            ));
        }
        if locators
            .iter()
            .any(|locator| !supplied.contains(&locator.resource_id()))
            || supplied.iter().any(|resource_id| {
                !locators
                    .iter()
                    .any(|item| item.resource_id() == *resource_id)
            })
        {
            return Err(Error::new(
                ErrorKind::InvalidArgument,
                "every import resource must own at least one supplied locator",
            ));
        }
        Ok(Self {
            asset,
            representation,
            resources,
            locators,
        })
    }

    /// Returns the logical asset.
    #[must_use]
    pub const fn asset(&self) -> &Asset {
        &self.asset
    }

    /// Returns the original representation.
    #[must_use]
    pub const fn representation(&self) -> &Representation {
        &self.representation
    }

    /// Returns the storage resources realizing the representation.
    #[must_use]
    pub fn resources(&self) -> &[Resource] {
        &self.resources
    }

    /// Returns the known access routes for the imported resources.
    #[must_use]
    pub fn locators(&self) -> &[Locator] {
        &self.locators
    }

    /// Splits the aggregate into persistable domain values.
    #[must_use]
    pub fn into_parts(self) -> (Asset, Representation, Vec<Resource>, Vec<Locator>) {
        (
            self.asset,
            self.representation,
            self.resources,
            self.locators,
        )
    }
}

/// An ordered filesystem or URI boundary searched by the resolver.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MediaRoot {
    id: MediaRootId,
    uri: String,
    label: Option<String>,
    priority: i32,
    enabled: bool,
}

impl MediaRoot {
    /// Creates a configured media root with a syntactically valid absolute URI.
    ///
    /// # Errors
    ///
    /// Returns [`ErrorKind::InvalidArgument`] when `uri` is invalid or relative.
    pub fn new(
        id: MediaRootId,
        uri: impl Into<String>,
        label: Option<String>,
        priority: i32,
        enabled: bool,
    ) -> Result<Self> {
        let uri = normalize_uri(uri, "media-root")?;
        Ok(Self {
            id,
            uri,
            label,
            priority,
            enabled,
        })
    }

    /// Returns the root's stable identity.
    #[must_use]
    pub const fn id(&self) -> MediaRootId {
        self.id
    }

    /// Returns the UTF-8 root URI.
    #[must_use]
    pub fn uri(&self) -> &str {
        &self.uri
    }

    /// Returns the optional user-facing label.
    #[must_use]
    pub fn label(&self) -> Option<&str> {
        self.label.as_deref()
    }

    /// Returns the resolver priority; lower values are considered first.
    #[must_use]
    pub const fn priority(&self) -> i32 {
        self.priority
    }

    /// Returns whether this root participates in resolution.
    #[must_use]
    pub const fn is_enabled(&self) -> bool {
        self.enabled
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{LocatorAvailability, LocatorId, ResourceId};

    #[test]
    fn rejects_empty_root_uris() {
        let root = MediaRoot::new(MediaRootId::new(), "", None, 0, true);

        assert_eq!(
            root.expect_err("empty URI must fail").kind(),
            ErrorKind::InvalidArgument
        );
    }

    #[test]
    fn media_roots_have_deterministic_priority_order() {
        let first_id = MediaRootId::from_bytes([1; 16]);
        let second_id = MediaRootId::from_bytes([2; 16]);
        let mut project = Project::new(ProjectId::new(), 1, Timestamp::from_unix_micros(0), None);
        project.set_media_roots(vec![
            MediaRoot::new(second_id, "file:///b", None, 10, true).expect("valid root"),
            MediaRoot::new(first_id, "file:///a", None, 10, true).expect("valid root"),
            MediaRoot::new(MediaRootId::new(), "file:///top", None, 0, true).expect("valid root"),
        ]);

        assert_eq!(project.media_roots()[0].priority(), 0);
        assert_eq!(project.media_roots()[1].id(), first_id);
        assert_eq!(project.media_roots()[2].id(), second_id);
    }

    #[test]
    fn fingerprint_validation_preserves_extensibility() {
        let fingerprint =
            Fingerprint::new("blake3", 1, vec![1, 2, 3]).expect("algorithm label is valid");
        assert_eq!(fingerprint.algorithm(), "blake3");
        assert_eq!(fingerprint.version(), 1);
        assert_eq!(fingerprint.value(), [1, 2, 3]);

        assert!(Fingerprint::new("contains spaces", 1, vec![1]).is_err());
        assert!(Fingerprint::new("valid", 1, Vec::new()).is_err());
    }

    #[test]
    fn import_aggregate_enforces_ownership() {
        let asset = Asset::new(AssetId::new(), Timestamp::from_unix_micros(0), None, None);
        let resource_id = ResourceId::new();
        let representation = Representation::new(
            RepresentationId::new(),
            AssetId::new(),
            RepresentationKind::Original,
            ContentStructure::single_resource(resource_id),
            Vec::new(),
        );
        let resource = Resource::new(resource_id, Vec::new(), None);
        let locator = Locator::new(
            LocatorId::new(),
            resource_id,
            "file:///media.mov",
            None,
            LocatorAvailability::Online,
        )
        .expect("valid locator");

        assert_eq!(
            OriginalMediaImport::new(asset, representation, vec![resource], vec![locator])
                .expect_err("mismatched ownership must fail")
                .kind(),
            ErrorKind::InvalidArgument
        );
    }
}
