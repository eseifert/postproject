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
    /// Stable identity of a project.
    ProjectId
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
    /// Stable identity of a physical media location record.
    LocationId
);
strong_id!(
    /// Stable identity of a configured resolver search root.
    MediaRootId
);
strong_id!(
    /// Stable identity of a domain transaction, reserved for later journaling.
    TransactionId
);

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;

    proptest! {
        #[test]
        fn project_id_byte_and_text_round_trips(bytes in any::<[u8; 16]>()) {
            let id = ProjectId::from_bytes(bytes);
            prop_assert_eq!(ProjectId::from_str(&id.to_string()), Ok(id));
            prop_assert_eq!(id.into_bytes(), bytes);
        }
    }

    #[test]
    fn identifier_types_are_not_interchangeable() {
        let bytes = [7; 16];
        let project = ProjectId::from_bytes(bytes);
        let asset = AssetId::from_bytes(bytes);

        assert_eq!(project.as_bytes(), asset.as_bytes());
        assert_eq!(project.to_string(), asset.to_string());
    }

    #[test]
    fn invalid_text_has_stable_error_kind() {
        let error = AssetId::from_str("not-a-uuid").expect_err("text must be rejected");
        assert_eq!(error.kind(), ErrorKind::InvalidArgument);
    }
}
