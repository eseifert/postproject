//! C-ABI-owned projection of recursive metadata values.

use std::ffi::CString;

use postproject_core::{
    Error, ErrorKind, MetadataAssertion, MetadataField, MetadataMatch, MetadataValue,
    MetadataValueKind, ObjectRef, QueryCursor,
};

use crate::{PpObjectRef, exact_cstring, object_ref_to_abi};

pub(crate) const PP_METADATA_STRING: u32 = 1;
pub(crate) const PP_METADATA_LANG_STRING: u32 = 2;
pub(crate) const PP_METADATA_I64: u32 = 3;
pub(crate) const PP_METADATA_U64: u32 = 4;
pub(crate) const PP_METADATA_DECIMAL: u32 = 5;
pub(crate) const PP_METADATA_BOOL: u32 = 6;
pub(crate) const PP_METADATA_TIMESTAMP: u32 = 7;
pub(crate) const PP_METADATA_URI: u32 = 8;
pub(crate) const PP_METADATA_BYTES: u32 = 9;
pub(crate) const PP_METADATA_RATIONAL: u32 = 10;
pub(crate) const PP_METADATA_LIST: u32 = 11;
pub(crate) const PP_METADATA_STRUCT: u32 = 12;
pub(crate) const PP_METADATA_REFERENCE: u32 = 13;

/// Opaque immutable metadata result set owned by the C caller.
pub struct PpMetadataSet {
    pub(crate) assertions: Vec<AbiMetadataAssertion>,
    pub(crate) next_cursor: Option<CString>,
}

pub(crate) struct AbiMetadataAssertion {
    pub(crate) target: PpObjectRef,
    pub(crate) vocabulary: CString,
    pub(crate) property: CString,
    pub(crate) value: PpMetadataValue,
}

/// Opaque immutable typed metadata value borrowed from a result set.
pub struct PpMetadataValue {
    pub(crate) inner: AbiMetadataValue,
}

pub(crate) enum AbiMetadataValue {
    String {
        value: CString,
        language: Option<CString>,
    },
    I64(i64),
    U64(u64),
    Decimal {
        coefficient: CString,
        scale: u32,
    },
    Bool(bool),
    Timestamp(i64),
    Uri(CString),
    Bytes(Vec<u8>),
    Rational {
        numerator: i64,
        denominator: u64,
    },
    List(Vec<PpMetadataValue>),
    Struct(Vec<AbiMetadataField>),
    Reference(PpObjectRef),
}

pub(crate) struct AbiMetadataField {
    pub(crate) name: CString,
    pub(crate) value: PpMetadataValue,
}

impl PpMetadataSet {
    pub(crate) fn from_assertions(
        target: ObjectRef,
        assertions: &[MetadataAssertion],
    ) -> Result<Self, Error> {
        let target = object_ref_to_abi(target)?;
        let assertions = assertions
            .iter()
            .map(|assertion| AbiMetadataAssertion::new(target, assertion))
            .collect::<Result<_, _>>()?;
        Ok(Self {
            assertions,
            next_cursor: None,
        })
    }

    pub(crate) fn from_matches(matches: &[MetadataMatch]) -> Result<Self, Error> {
        let assertions = matches
            .iter()
            .map(|matched| {
                AbiMetadataAssertion::new(object_ref_to_abi(matched.target())?, matched.assertion())
            })
            .collect::<Result<_, _>>()?;
        Ok(Self {
            assertions,
            next_cursor: None,
        })
    }

    pub(crate) fn from_page(
        matches: &[MetadataMatch],
        next_cursor: Option<&QueryCursor>,
    ) -> Result<Self, Error> {
        let mut set = Self::from_matches(matches)?;
        set.next_cursor = next_cursor
            .map(|cursor| exact_cstring(cursor.as_str(), "metadata query cursor"))
            .transpose()?;
        Ok(set)
    }
}

impl AbiMetadataAssertion {
    fn new(target: PpObjectRef, assertion: &MetadataAssertion) -> Result<Self, Error> {
        Ok(Self {
            target,
            vocabulary: exact_cstring(
                assertion.property().vocabulary().as_str(),
                "metadata vocabulary",
            )?,
            property: exact_cstring(
                assertion.property().property().as_str(),
                "metadata property",
            )?,
            value: PpMetadataValue::try_from(assertion.value())?,
        })
    }
}

impl PpMetadataValue {
    pub(crate) const fn kind(&self) -> u32 {
        match self.inner {
            AbiMetadataValue::String { language: None, .. } => PP_METADATA_STRING,
            AbiMetadataValue::String {
                language: Some(_), ..
            } => PP_METADATA_LANG_STRING,
            AbiMetadataValue::I64(_) => PP_METADATA_I64,
            AbiMetadataValue::U64(_) => PP_METADATA_U64,
            AbiMetadataValue::Decimal { .. } => PP_METADATA_DECIMAL,
            AbiMetadataValue::Bool(_) => PP_METADATA_BOOL,
            AbiMetadataValue::Timestamp(_) => PP_METADATA_TIMESTAMP,
            AbiMetadataValue::Uri(_) => PP_METADATA_URI,
            AbiMetadataValue::Bytes(_) => PP_METADATA_BYTES,
            AbiMetadataValue::Rational { .. } => PP_METADATA_RATIONAL,
            AbiMetadataValue::List(_) => PP_METADATA_LIST,
            AbiMetadataValue::Struct(_) => PP_METADATA_STRUCT,
            AbiMetadataValue::Reference(_) => PP_METADATA_REFERENCE,
        }
    }
}

impl TryFrom<&MetadataValue> for PpMetadataValue {
    type Error = Error;

    #[allow(
        clippy::too_many_lines,
        reason = "keeping the exhaustive domain-to-ABI kind mapping together is auditable"
    )]
    fn try_from(value: &MetadataValue) -> Result<Self, Self::Error> {
        let inner = match value.kind() {
            MetadataValueKind::String => AbiMetadataValue::String {
                value: exact_cstring(
                    value
                        .as_string()
                        .ok_or_else(|| conversion_error("string"))?,
                    "metadata string",
                )?,
                language: None,
            },
            MetadataValueKind::LangString => {
                let (text, language) = value
                    .as_language_string()
                    .ok_or_else(|| conversion_error("language string"))?;
                AbiMetadataValue::String {
                    value: exact_cstring(text, "metadata language string")?,
                    language: Some(exact_cstring(language, "metadata language tag")?),
                }
            }
            MetadataValueKind::I64 => {
                AbiMetadataValue::I64(value.as_i64().ok_or_else(|| conversion_error("i64"))?)
            }
            MetadataValueKind::U64 => {
                AbiMetadataValue::U64(value.as_u64().ok_or_else(|| conversion_error("u64"))?)
            }
            MetadataValueKind::Decimal => {
                let decimal = value
                    .as_decimal()
                    .ok_or_else(|| conversion_error("decimal"))?;
                AbiMetadataValue::Decimal {
                    coefficient: exact_cstring(
                        &decimal.coefficient().to_string(),
                        "metadata decimal coefficient",
                    )?,
                    scale: decimal.scale(),
                }
            }
            MetadataValueKind::Bool => {
                AbiMetadataValue::Bool(value.as_bool().ok_or_else(|| conversion_error("bool"))?)
            }
            MetadataValueKind::Timestamp => AbiMetadataValue::Timestamp(
                value
                    .as_timestamp()
                    .ok_or_else(|| conversion_error("timestamp"))?
                    .as_unix_micros(),
            ),
            MetadataValueKind::Uri => AbiMetadataValue::Uri(exact_cstring(
                value.as_uri().ok_or_else(|| conversion_error("URI"))?,
                "metadata URI",
            )?),
            MetadataValueKind::Bytes => AbiMetadataValue::Bytes(
                value
                    .as_bytes()
                    .ok_or_else(|| conversion_error("bytes"))?
                    .to_vec(),
            ),
            MetadataValueKind::Rational => {
                let rational = value
                    .as_rational()
                    .ok_or_else(|| conversion_error("rational"))?;
                AbiMetadataValue::Rational {
                    numerator: rational.numerator(),
                    denominator: rational.denominator(),
                }
            }
            MetadataValueKind::List => AbiMetadataValue::List(
                value
                    .as_list()
                    .ok_or_else(|| conversion_error("list"))?
                    .iter()
                    .map(PpMetadataValue::try_from)
                    .collect::<Result<_, _>>()?,
            ),
            MetadataValueKind::Struct => AbiMetadataValue::Struct(
                value
                    .as_structure()
                    .ok_or_else(|| conversion_error("structure"))?
                    .iter()
                    .map(AbiMetadataField::try_from)
                    .collect::<Result<_, _>>()?,
            ),
            MetadataValueKind::Reference => AbiMetadataValue::Reference(object_ref_to_abi(
                value
                    .as_reference()
                    .ok_or_else(|| conversion_error("reference"))?,
            )?),
            _ => {
                return Err(Error::new(
                    ErrorKind::Unsupported,
                    "metadata value kind is not supported by this ABI",
                ));
            }
        };
        Ok(Self { inner })
    }
}

impl TryFrom<&MetadataField> for AbiMetadataField {
    type Error = Error;

    fn try_from(field: &MetadataField) -> Result<Self, Self::Error> {
        Ok(Self {
            name: exact_cstring(field.name().as_str(), "metadata field name")?,
            value: PpMetadataValue::try_from(field.value())?,
        })
    }
}

fn conversion_error(expected: &str) -> Error {
    Error::new(
        ErrorKind::Internal,
        format!("metadata {expected} kind has no matching value"),
    )
}
