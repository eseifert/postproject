//! Strong, globally unique identifiers.

use std::{fmt, str::FromStr};

use uuid::Uuid;

use crate::{Error, ErrorKind, Result};

macro_rules! strong_id {
    ($(#[$metadata:meta])* $name:ident) => {
        $(#[$metadata])*
        #[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub struct $name(Uuid);

        impl $name {
            /// Creates a random RFC 9562 UUID version 4 identity.
            #[must_use]
            pub fn new() -> Self {
                Self(Uuid::new_v4())
            }

            /// Creates an identity from its stable 16-byte representation.
            #[must_use]
            pub const fn from_bytes(bytes: [u8; 16]) -> Self {
                Self(Uuid::from_bytes(bytes))
            }

            /// Returns the stable 16-byte representation.
            #[must_use]
            pub const fn into_bytes(self) -> [u8; 16] {
                self.0.into_bytes()
            }

            /// Returns a reference to the stable 16-byte representation.
            #[must_use]
            pub const fn as_bytes(&self) -> &[u8; 16] {
                self.0.as_bytes()
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.0.fmt(formatter)
            }
        }

        impl FromStr for $name {
            type Err = Error;

            fn from_str(value: &str) -> Result<Self> {
                Uuid::parse_str(value).map(Self).map_err(|error| {
                    Error::new(
                        ErrorKind::InvalidArgument,
                        format!("invalid {}: {error}", stringify!($name)),
                    )
                })
            }
        }
    };
}

strong_id!(
    /// Stable identity of a production.
    ProductionId
);
strong_id!(
    /// Stable logical identity of an asset.
    AssetId
);
strong_id!(
    /// Stable identity of an encoded or derived asset representation.
    RepresentationId
);
strong_id!(
    /// Stable identity of stored content used by a representation.
    ResourceId
);
strong_id!(
    /// Stable identity of one access route to a resource.
    LocatorId
);
strong_id!(
    /// Stable identity of a configured resolver search root.
    MediaRootId
);
strong_id!(
    /// Stable identity of a production activity.
    ActivityId
);
strong_id!(
    /// Stable identity of one requested production job.
    JobId
);
strong_id!(
    /// Capability identifying one active job claim.
    JobClaimId
);
strong_id!(
    /// Stable identity of one durable production revision.
    RevisionId
);
strong_id!(
    /// Stable identity of a domain transaction.
    TransactionId
);

/// A typed reference to an object that may carry extensible assertions.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[non_exhaustive]
pub enum ObjectRef {
    /// A production object.
    Production(ProductionId),
    /// A logical asset object.
    Asset(AssetId),
    /// A concrete asset representation.
    Representation(RepresentationId),
    /// A storage-level resource.
    Resource(ResourceId),
    /// A production activity.
    Activity(ActivityId),
    /// A requested production job.
    Job(JobId),
}

/// Portable reference from a host document to one production-scoped object.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct HostObjectBinding {
    production_id: ProductionId,
    object: ObjectRef,
}

impl HostObjectBinding {
    /// Creates a binding from its authoritative identity tuple.
    ///
    /// # Errors
    ///
    /// Returns an error when a production object does not match the binding's
    /// containing production identity.
    pub fn new(production_id: ProductionId, object: ObjectRef) -> Result<Self> {
        if let ObjectRef::Production(object_id) = object {
            if object_id != production_id {
                return Err(Error::new(
                    ErrorKind::InvalidArgument,
                    "a production binding must reference its containing production",
                ));
            }
        }
        Ok(Self {
            production_id,
            object,
        })
    }

    /// Returns the production that scopes the referenced object identity.
    #[must_use]
    pub const fn production_id(self) -> ProductionId {
        self.production_id
    }

    /// Returns the typed object reference within the production.
    #[must_use]
    pub const fn object(self) -> ObjectRef {
        self.object
    }
}

impl fmt::Display for HostObjectBinding {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (kind, object_id) = match self.object {
            ObjectRef::Production(id) => ("production", id.to_string()),
            ObjectRef::Asset(id) => ("asset", id.to_string()),
            ObjectRef::Representation(id) => ("representation", id.to_string()),
            ObjectRef::Resource(id) => ("resource", id.to_string()),
            ObjectRef::Activity(id) => ("activity", id.to_string()),
            ObjectRef::Job(id) => ("job", id.to_string()),
        };
        write!(
            formatter,
            "https://postproject.org/ref/v1/{}/{kind}/{object_id}",
            self.production_id
        )
    }
}

impl FromStr for HostObjectBinding {
    type Err = Error;

    fn from_str(value: &str) -> Result<Self> {
        let Some(path) = value.strip_prefix("https://postproject.org/ref/v1/") else {
            return Err(invalid_binding());
        };
        let mut parts = path.split('/');
        let (Some(production_id), Some(kind), Some(object_id)) =
            (parts.next(), parts.next(), parts.next())
        else {
            return Err(invalid_binding());
        };
        if parts.next().is_some() {
            return Err(invalid_binding());
        }
        let production_id = parse_canonical_id::<ProductionId>(production_id, "production UUID")?;
        let object = match kind {
            "production" => ObjectRef::Production(parse_canonical_id(object_id, "object UUID")?),
            "asset" => ObjectRef::Asset(parse_canonical_id(object_id, "object UUID")?),
            "representation" => {
                ObjectRef::Representation(parse_canonical_id(object_id, "object UUID")?)
            }
            "resource" => ObjectRef::Resource(parse_canonical_id(object_id, "object UUID")?),
            "activity" => ObjectRef::Activity(parse_canonical_id(object_id, "object UUID")?),
            "job" => ObjectRef::Job(parse_canonical_id(object_id, "object UUID")?),
            _ => return Err(invalid_binding()),
        };
        Self::new(production_id, object)
    }
}

fn parse_canonical_id<T>(value: &str, label: &str) -> Result<T>
where
    T: FromStr<Err = Error> + fmt::Display,
{
    let parsed = T::from_str(value)?;
    if parsed.to_string() != value {
        return Err(Error::new(
            ErrorKind::InvalidArgument,
            format!("host binding {label} must use canonical lowercase UUID text"),
        ));
    }
    Ok(parsed)
}

fn invalid_binding() -> Error {
    Error::new(
        ErrorKind::InvalidArgument,
        "host binding must be https://postproject.org/ref/v1/<production UUID>/<object kind>/<object UUID>",
    )
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;

    proptest! {
        #[test]
        fn production_id_byte_and_text_round_trips(bytes in any::<[u8; 16]>()) {
            let id = ProductionId::from_bytes(bytes);
            prop_assert_eq!(ProductionId::from_str(&id.to_string()), Ok(id));
            prop_assert_eq!(id.into_bytes(), bytes);
        }
    }

    #[test]
    fn identifier_types_are_not_interchangeable() {
        let bytes = [7; 16];
        let production = ProductionId::from_bytes(bytes);
        let asset = AssetId::from_bytes(bytes);
        let resource = ResourceId::from_bytes(bytes);
        let locator = LocatorId::from_bytes(bytes);

        assert_eq!(production.as_bytes(), asset.as_bytes());
        assert_eq!(production.to_string(), asset.to_string());
        assert_eq!(resource.as_bytes(), locator.as_bytes());
    }

    #[test]
    fn object_references_preserve_identity_level() {
        let bytes = [9; 16];
        assert_ne!(
            ObjectRef::Asset(AssetId::from_bytes(bytes)),
            ObjectRef::Representation(RepresentationId::from_bytes(bytes))
        );
    }

    #[test]
    fn invalid_text_has_stable_error_kind() {
        let error = AssetId::from_str("not-a-uuid").expect_err("text must be rejected");
        assert_eq!(error.kind(), ErrorKind::InvalidArgument);
    }

    #[test]
    fn host_bindings_round_trip_each_object_kind() {
        let production_id = ProductionId::from_bytes([1; 16]);
        let objects = [
            ObjectRef::Production(production_id),
            ObjectRef::Asset(AssetId::from_bytes([2; 16])),
            ObjectRef::Representation(RepresentationId::from_bytes([3; 16])),
            ObjectRef::Resource(ResourceId::from_bytes([4; 16])),
            ObjectRef::Activity(ActivityId::from_bytes([5; 16])),
            ObjectRef::Job(JobId::from_bytes([6; 16])),
        ];
        for object in objects {
            let binding = HostObjectBinding::new(production_id, object).expect("valid binding");
            let encoded = binding.to_string();
            assert_eq!(HostObjectBinding::from_str(&encoded), Ok(binding));
        }
    }

    #[test]
    fn host_bindings_reject_noncanonical_or_ambiguous_text() {
        let production_id = ProductionId::from_bytes([1; 16]);
        let asset_id = AssetId::from_bytes([2; 16]);
        let valid = format!("https://postproject.org/ref/v1/{production_id}/asset/{asset_id}");
        assert!(HostObjectBinding::from_str(&valid.to_uppercase()).is_err());
        assert!(HostObjectBinding::from_str(&valid.replace("/v1/", "/v2/")).is_err());
        assert!(HostObjectBinding::from_str(&format!("{valid}/fallback")).is_err());
        assert!(HostObjectBinding::from_str(&valid.replace("/asset/", "/locator/")).is_err());
        assert!(HostObjectBinding::from_str("postproject:v1:old:asset:old").is_err());
        assert!(
            HostObjectBinding::new(
                production_id,
                ObjectRef::Production(ProductionId::from_bytes([9; 16]))
            )
            .is_err()
        );
    }
}
