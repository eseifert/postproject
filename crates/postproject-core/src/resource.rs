//! Storage-level resource identity and access values.

use crate::{FileFacts, Fingerprint, LocatorId, ResourceId, Result, Timestamp, uri::normalize_uri};

macro_rules! typed_fingerprint {
    ($(#[$metadata:meta])* $name:ident) => {
        $(#[$metadata])*
        #[derive(Clone, Debug, Eq, PartialEq)]
        pub struct $name(Fingerprint);

        impl $name {
            /// Creates typed fingerprint evidence.
            ///
            /// # Errors
            ///
            /// Returns an error when the algorithm or value is invalid.
            pub fn new(
                algorithm: impl Into<String>,
                version: u16,
                value: Vec<u8>,
            ) -> Result<Self> {
                Fingerprint::new(algorithm, version, value).map(Self)
            }

            /// Returns the algorithm identifier.
            #[must_use]
            pub fn algorithm(&self) -> &str {
                self.0.algorithm()
            }

            /// Returns the algorithm format version.
            #[must_use]
            pub const fn version(&self) -> u16 {
                self.0.version()
            }

            /// Returns the opaque fingerprint bytes.
            #[must_use]
            pub fn value(&self) -> &[u8] {
                self.0.value()
            }
        }
    };
}

typed_fingerprint!(
    /// Versioned identity evidence derived from one storage resource.
    ResourceFingerprint
);
typed_fingerprint!(
    /// Versioned, structure-aware identity evidence for a representation.
    RepresentationFingerprint
);

/// A storage-level component used to realize a representation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Resource {
    id: ResourceId,
    fingerprints: Vec<ResourceFingerprint>,
    file_facts: Option<FileFacts>,
}

impl Resource {
    /// Creates a resource from content evidence known at observation time.
    #[must_use]
    pub fn new(
        id: ResourceId,
        fingerprints: Vec<ResourceFingerprint>,
        file_facts: Option<FileFacts>,
    ) -> Self {
        Self {
            id,
            fingerprints,
            file_facts,
        }
    }

    /// Returns the resource's stable identity.
    #[must_use]
    pub const fn id(&self) -> ResourceId {
        self.id
    }

    /// Returns stored content identity evidence.
    #[must_use]
    pub fn fingerprints(&self) -> &[ResourceFingerprint] {
        &self.fingerprints
    }

    /// Returns cheap stored file facts, when available.
    #[must_use]
    pub const fn file_facts(&self) -> Option<FileFacts> {
        self.file_facts
    }
}

/// The last observed availability of a resource locator.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum LocatorAvailability {
    /// Availability has not been checked.
    Unknown,
    /// The locator resolved when last checked.
    Online,
    /// The locator did not resolve when last checked.
    Offline,
}

/// A URI identifying one access route to a resource.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Locator {
    id: LocatorId,
    resource_id: ResourceId,
    uri: String,
    last_seen: Option<Timestamp>,
    availability: LocatorAvailability,
}

impl Locator {
    /// Creates a locator with a syntactically valid absolute URI.
    ///
    /// # Errors
    ///
    /// Returns an error when `uri` is invalid or relative.
    pub fn new(
        id: LocatorId,
        resource_id: ResourceId,
        uri: impl Into<String>,
        last_seen: Option<Timestamp>,
        availability: LocatorAvailability,
    ) -> Result<Self> {
        let uri = normalize_uri(uri, "locator")?;
        Ok(Self {
            id,
            resource_id,
            uri,
            last_seen,
            availability,
        })
    }

    /// Returns the locator's stable identity.
    #[must_use]
    pub const fn id(&self) -> LocatorId {
        self.id
    }

    /// Returns the resource made accessible by this locator.
    #[must_use]
    pub const fn resource_id(&self) -> ResourceId {
        self.resource_id
    }

    /// Returns the UTF-8 URI.
    #[must_use]
    pub fn uri(&self) -> &str {
        &self.uri
    }

    /// Returns when the locator was last observed online.
    #[must_use]
    pub const fn last_seen(&self) -> Option<Timestamp> {
        self.last_seen
    }

    /// Returns its last observed availability.
    #[must_use]
    pub const fn availability(&self) -> LocatorAvailability {
        self.availability
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn locator_belongs_to_a_resource() {
        let resource_id = ResourceId::new();
        let locator = Locator::new(
            LocatorId::new(),
            resource_id,
            "file:///media/clip.mov",
            None,
            LocatorAvailability::Unknown,
        )
        .expect("valid locator");

        assert_eq!(locator.resource_id(), resource_id);
        assert_eq!(locator.uri(), "file:///media/clip.mov");
    }

    #[test]
    fn fingerprint_domains_are_explicit() {
        let resource = ResourceFingerprint::new("blake3", 1, vec![1]).expect("valid");
        let representation =
            RepresentationFingerprint::new("tree-blake3", 1, vec![2]).expect("valid");

        assert_eq!(resource.algorithm(), "blake3");
        assert_eq!(representation.algorithm(), "tree-blake3");
    }
}
