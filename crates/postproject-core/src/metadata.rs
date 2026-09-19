//! Standards-aware, typed metadata values.

use url::Url;

use crate::{Error, ErrorKind, ObjectRef, Result, Timestamp};

/// Maximum UTF-8 byte length of a vocabulary identifier.
pub const MAX_VOCABULARY_ID_BYTES: usize = 512;
/// Maximum UTF-8 byte length of a vocabulary-local property identifier.
pub const MAX_PROPERTY_ID_BYTES: usize = 255;
/// Maximum UTF-8 byte length of one text value.
pub const MAX_METADATA_TEXT_BYTES: usize = 1024 * 1024;
/// Maximum UTF-8 byte length of one URI value.
pub const MAX_METADATA_URI_BYTES: usize = 4096;
/// Maximum byte length of one binary value.
pub const MAX_METADATA_BINARY_BYTES: usize = 16 * 1024 * 1024;
/// Maximum byte length of a language tag.
pub const MAX_LANGUAGE_TAG_BYTES: usize = 64;
/// Maximum number of direct children in a list or structured value.
pub const MAX_METADATA_COLLECTION_ITEMS: usize = 4096;
/// Maximum number of fractional decimal digits.
pub const MAX_METADATA_DECIMAL_SCALE: u32 = 1024;
/// Maximum nesting depth of lists and structured values, including the root.
pub const MAX_METADATA_NESTING_DEPTH: usize = 32;
/// Maximum aggregate payload size of one metadata value.
pub const MAX_METADATA_TOTAL_BYTES: usize = 16 * 1024 * 1024;

/// An extensible identifier for a metadata vocabulary or namespace.
///
/// Core preserves the caller's exact identifier. A vocabulary registry may
/// interpret it, but a registry is not required to store unknown metadata.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct VocabularyId(String);

impl VocabularyId {
    /// Creates a bounded vocabulary identifier.
    ///
    /// # Errors
    ///
    /// Returns [`ErrorKind::InvalidArgument`] for empty, oversized, or
    /// whitespace/control-containing input.
    pub fn new(value: impl Into<String>) -> Result<Self> {
        let value = value.into();
        validate_identifier("metadata vocabulary", &value, MAX_VOCABULARY_ID_BYTES)?;
        Ok(Self(value))
    }

    /// Returns the exact identifier supplied by the caller.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Consumes the identifier and returns its string.
    #[must_use]
    pub fn into_string(self) -> String {
        self.0
    }
}

/// A property name local to a metadata vocabulary.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct PropertyId(String);

impl PropertyId {
    /// Creates a bounded vocabulary-local property identifier.
    ///
    /// # Errors
    ///
    /// Returns [`ErrorKind::InvalidArgument`] for empty, oversized, or
    /// whitespace/control-containing input.
    pub fn new(value: impl Into<String>) -> Result<Self> {
        let value = value.into();
        validate_identifier("metadata property", &value, MAX_PROPERTY_ID_BYTES)?;
        Ok(Self(value))
    }

    /// Returns the exact identifier supplied by the caller.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Consumes the identifier and returns its string.
    #[must_use]
    pub fn into_string(self) -> String {
        self.0
    }
}

/// A globally meaningful property formed from a vocabulary and local name.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct MetadataProperty {
    vocabulary: VocabularyId,
    property: PropertyId,
}

impl MetadataProperty {
    /// Creates a metadata property identity.
    #[must_use]
    pub const fn new(vocabulary: VocabularyId, property: PropertyId) -> Self {
        Self {
            vocabulary,
            property,
        }
    }

    /// Returns the vocabulary or namespace identity.
    #[must_use]
    pub const fn vocabulary(&self) -> &VocabularyId {
        &self.vocabulary
    }

    /// Returns the vocabulary-local property identity.
    #[must_use]
    pub const fn property(&self) -> &PropertyId {
        &self.property
    }
}

/// An exact base-ten decimal represented as `coefficient * 10^-scale`.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct DecimalValue {
    coefficient: i128,
    scale: u32,
}

impl DecimalValue {
    /// Creates a decimal and removes insignificant trailing fractional zeros.
    ///
    /// # Errors
    ///
    /// Returns [`ErrorKind::InvalidArgument`] when the normalized scale
    /// exceeds [`MAX_METADATA_DECIMAL_SCALE`].
    pub fn new(mut coefficient: i128, mut scale: u32) -> Result<Self> {
        while scale > 0 && coefficient % 10 == 0 {
            coefficient /= 10;
            scale -= 1;
        }
        if scale > MAX_METADATA_DECIMAL_SCALE {
            return Err(limit_error(
                "metadata decimal scale",
                MAX_METADATA_DECIMAL_SCALE as usize,
            ));
        }
        Ok(Self { coefficient, scale })
    }

    /// Returns the signed base-ten coefficient.
    #[must_use]
    pub const fn coefficient(self) -> i128 {
        self.coefficient
    }

    /// Returns the number of fractional decimal digits.
    #[must_use]
    pub const fn scale(self) -> u32 {
        self.scale
    }
}

/// An exact normalized rational number.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct RationalValue {
    numerator: i64,
    denominator: u64,
}

impl RationalValue {
    /// Creates a rational value reduced to lowest terms.
    ///
    /// # Errors
    ///
    /// Returns [`ErrorKind::InvalidArgument`] when `denominator` is zero.
    pub fn new(numerator: i64, denominator: u64) -> Result<Self> {
        if denominator == 0 {
            return Err(Error::new(
                ErrorKind::InvalidArgument,
                "metadata rational denominator must not be zero",
            ));
        }
        let divisor = greatest_common_divisor(numerator.unsigned_abs(), denominator);
        let normalized_numerator = match i64::try_from(divisor) {
            Ok(divisor) => numerator / divisor,
            Err(_) => -1,
        };
        Ok(Self {
            numerator: normalized_numerator,
            denominator: denominator / divisor,
        })
    }

    /// Returns the normalized numerator.
    #[must_use]
    pub const fn numerator(self) -> i64 {
        self.numerator
    }

    /// Returns the positive normalized denominator.
    #[must_use]
    pub const fn denominator(self) -> u64 {
        self.denominator
    }
}

/// The inspectable type of a [`MetadataValue`].
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum MetadataValueKind {
    /// Unqualified UTF-8 text.
    String,
    /// UTF-8 text with a language tag.
    LangString,
    /// Signed 64-bit integer.
    I64,
    /// Unsigned 64-bit integer.
    U64,
    /// Exact base-ten decimal.
    Decimal,
    /// Boolean.
    Bool,
    /// Signed Unix-microsecond timestamp.
    Timestamp,
    /// URI text validated without normalization.
    Uri,
    /// Opaque bytes.
    Bytes,
    /// Exact rational number.
    Rational,
    /// Ordered nested values.
    List,
    /// Ordered named fields.
    Struct,
    /// Reference to another `PostProject` object.
    Reference,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
enum MetadataValueInner {
    String(String),
    LangString { value: String, language: String },
    I64(i64),
    U64(u64),
    Decimal(DecimalValue),
    Bool(bool),
    Timestamp(Timestamp),
    Uri(String),
    Bytes(Vec<u8>),
    Rational(RationalValue),
    List(Vec<MetadataValue>),
    Struct(Vec<MetadataField>),
    Reference(ObjectRef),
}

/// A bounded, recursively typed metadata value.
///
/// Constructors preserve ordering and exact text while enforcing limits. The
/// private representation prevents callers from bypassing those invariants.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct MetadataValue {
    inner: MetadataValueInner,
    depth: usize,
    payload_bytes: usize,
}

impl MetadataValue {
    /// Creates an unqualified text value.
    ///
    /// # Errors
    ///
    /// Returns [`ErrorKind::InvalidArgument`] when the text exceeds its byte
    /// limit or contains NUL.
    pub fn string(value: impl Into<String>) -> Result<Self> {
        let value = value.into();
        validate_text("metadata string", &value, MAX_METADATA_TEXT_BYTES)?;
        let payload_bytes = value.len();
        Ok(Self::leaf(MetadataValueInner::String(value), payload_bytes))
    }

    /// Creates language-tagged text with a syntax-safe BCP 47-shaped tag.
    ///
    /// # Errors
    ///
    /// Returns [`ErrorKind::InvalidArgument`] when the text is invalid or the
    /// language tag is empty, oversized, or not syntactically safe.
    pub fn language_string(value: impl Into<String>, language: impl Into<String>) -> Result<Self> {
        let value = value.into();
        let language = language.into();
        validate_text("metadata language string", &value, MAX_METADATA_TEXT_BYTES)?;
        validate_language_tag(&language)?;
        let payload_bytes = checked_payload_sum([value.len(), language.len()])?;
        Ok(Self::leaf(
            MetadataValueInner::LangString { value, language },
            payload_bytes,
        ))
    }

    /// Creates a signed integer value.
    #[must_use]
    pub const fn i64(value: i64) -> Self {
        Self::leaf(MetadataValueInner::I64(value), size_of::<i64>())
    }

    /// Creates an unsigned integer value.
    #[must_use]
    pub const fn u64(value: u64) -> Self {
        Self::leaf(MetadataValueInner::U64(value), size_of::<u64>())
    }

    /// Creates an exact decimal value.
    #[must_use]
    pub const fn decimal(value: DecimalValue) -> Self {
        Self::leaf(
            MetadataValueInner::Decimal(value),
            size_of::<i128>() + size_of::<u32>(),
        )
    }

    /// Creates a boolean value.
    #[must_use]
    pub const fn boolean(value: bool) -> Self {
        Self::leaf(MetadataValueInner::Bool(value), 1)
    }

    /// Creates a timestamp value.
    #[must_use]
    pub const fn timestamp(value: Timestamp) -> Self {
        Self::leaf(MetadataValueInner::Timestamp(value), size_of::<i64>())
    }

    /// Creates a URI value while preserving its exact spelling.
    ///
    /// # Errors
    ///
    /// Returns [`ErrorKind::InvalidArgument`] when the URI is invalid,
    /// oversized, or contains NUL.
    pub fn uri(value: impl Into<String>) -> Result<Self> {
        let value = value.into();
        validate_text("metadata URI", &value, MAX_METADATA_URI_BYTES)?;
        Url::parse(&value).map_err(|error| {
            Error::new(
                ErrorKind::InvalidArgument,
                format!("metadata URI is invalid: {error}"),
            )
        })?;
        let payload_bytes = value.len();
        Ok(Self::leaf(MetadataValueInner::Uri(value), payload_bytes))
    }

    /// Creates an opaque binary value.
    ///
    /// # Errors
    ///
    /// Returns [`ErrorKind::InvalidArgument`] when the value exceeds
    /// [`MAX_METADATA_BINARY_BYTES`].
    pub fn bytes(value: Vec<u8>) -> Result<Self> {
        if value.len() > MAX_METADATA_BINARY_BYTES {
            return Err(limit_error(
                "metadata binary value",
                MAX_METADATA_BINARY_BYTES,
            ));
        }
        let payload_bytes = value.len();
        Ok(Self::leaf(MetadataValueInner::Bytes(value), payload_bytes))
    }

    /// Creates an exact rational value.
    #[must_use]
    pub const fn rational(value: RationalValue) -> Self {
        Self::leaf(
            MetadataValueInner::Rational(value),
            size_of::<i64>() + size_of::<u64>(),
        )
    }

    /// Creates an ordered list.
    ///
    /// # Errors
    ///
    /// Returns [`ErrorKind::InvalidArgument`] when the item count, aggregate
    /// payload size, or nesting depth exceeds its documented limit.
    pub fn list(values: Vec<Self>) -> Result<Self> {
        if values.len() > MAX_METADATA_COLLECTION_ITEMS {
            return Err(limit_error(
                "metadata list item count",
                MAX_METADATA_COLLECTION_ITEMS,
            ));
        }
        let depth = composite_depth(values.iter().map(|value| value.depth))?;
        let payload_bytes = checked_payload_sum(values.iter().map(|value| value.payload_bytes))?;
        Ok(Self {
            inner: MetadataValueInner::List(values),
            depth,
            payload_bytes,
        })
    }

    /// Creates an ordered structured value.
    ///
    /// # Errors
    ///
    /// Returns [`ErrorKind::InvalidArgument`] when the field count, aggregate
    /// payload size, or nesting depth exceeds its documented limit.
    pub fn structure(fields: Vec<MetadataField>) -> Result<Self> {
        if fields.len() > MAX_METADATA_COLLECTION_ITEMS {
            return Err(limit_error(
                "metadata structure field count",
                MAX_METADATA_COLLECTION_ITEMS,
            ));
        }
        let depth = composite_depth(fields.iter().map(|field| field.value.depth))?;
        let payload_bytes = checked_payload_sum(
            fields
                .iter()
                .flat_map(|field| [field.name.as_str().len(), field.value.payload_bytes]),
        )?;
        Ok(Self {
            inner: MetadataValueInner::Struct(fields),
            depth,
            payload_bytes,
        })
    }

    /// Creates a reference to another `PostProject` object.
    #[must_use]
    pub const fn reference(value: ObjectRef) -> Self {
        Self::leaf(MetadataValueInner::Reference(value), 17)
    }

    /// Returns the value's inspectable type.
    #[must_use]
    pub const fn kind(&self) -> MetadataValueKind {
        match self.inner {
            MetadataValueInner::String(_) => MetadataValueKind::String,
            MetadataValueInner::LangString { .. } => MetadataValueKind::LangString,
            MetadataValueInner::I64(_) => MetadataValueKind::I64,
            MetadataValueInner::U64(_) => MetadataValueKind::U64,
            MetadataValueInner::Decimal(_) => MetadataValueKind::Decimal,
            MetadataValueInner::Bool(_) => MetadataValueKind::Bool,
            MetadataValueInner::Timestamp(_) => MetadataValueKind::Timestamp,
            MetadataValueInner::Uri(_) => MetadataValueKind::Uri,
            MetadataValueInner::Bytes(_) => MetadataValueKind::Bytes,
            MetadataValueInner::Rational(_) => MetadataValueKind::Rational,
            MetadataValueInner::List(_) => MetadataValueKind::List,
            MetadataValueInner::Struct(_) => MetadataValueKind::Struct,
            MetadataValueInner::Reference(_) => MetadataValueKind::Reference,
        }
    }

    /// Returns unqualified text, or `None` for another type.
    #[must_use]
    pub fn as_string(&self) -> Option<&str> {
        match &self.inner {
            MetadataValueInner::String(value) => Some(value),
            _ => None,
        }
    }

    /// Returns language-tagged text and its tag, or `None` for another type.
    #[must_use]
    pub fn as_language_string(&self) -> Option<(&str, &str)> {
        match &self.inner {
            MetadataValueInner::LangString { value, language } => Some((value, language)),
            _ => None,
        }
    }

    /// Returns a signed integer, or `None` for another type.
    #[must_use]
    pub const fn as_i64(&self) -> Option<i64> {
        match self.inner {
            MetadataValueInner::I64(value) => Some(value),
            _ => None,
        }
    }

    /// Returns an unsigned integer, or `None` for another type.
    #[must_use]
    pub const fn as_u64(&self) -> Option<u64> {
        match self.inner {
            MetadataValueInner::U64(value) => Some(value),
            _ => None,
        }
    }

    /// Returns an exact decimal, or `None` for another type.
    #[must_use]
    pub const fn as_decimal(&self) -> Option<DecimalValue> {
        match self.inner {
            MetadataValueInner::Decimal(value) => Some(value),
            _ => None,
        }
    }

    /// Returns a boolean, or `None` for another type.
    #[must_use]
    pub const fn as_bool(&self) -> Option<bool> {
        match self.inner {
            MetadataValueInner::Bool(value) => Some(value),
            _ => None,
        }
    }

    /// Returns a timestamp, or `None` for another type.
    #[must_use]
    pub const fn as_timestamp(&self) -> Option<Timestamp> {
        match self.inner {
            MetadataValueInner::Timestamp(value) => Some(value),
            _ => None,
        }
    }

    /// Returns URI text, or `None` for another type.
    #[must_use]
    pub fn as_uri(&self) -> Option<&str> {
        match &self.inner {
            MetadataValueInner::Uri(value) => Some(value),
            _ => None,
        }
    }

    /// Returns opaque bytes, or `None` for another type.
    #[must_use]
    pub fn as_bytes(&self) -> Option<&[u8]> {
        match &self.inner {
            MetadataValueInner::Bytes(value) => Some(value),
            _ => None,
        }
    }

    /// Returns an exact rational, or `None` for another type.
    #[must_use]
    pub const fn as_rational(&self) -> Option<RationalValue> {
        match self.inner {
            MetadataValueInner::Rational(value) => Some(value),
            _ => None,
        }
    }

    /// Returns ordered list items, or `None` for another type.
    #[must_use]
    pub fn as_list(&self) -> Option<&[Self]> {
        match &self.inner {
            MetadataValueInner::List(values) => Some(values),
            _ => None,
        }
    }

    /// Returns ordered structured fields, or `None` for another type.
    #[must_use]
    pub fn as_structure(&self) -> Option<&[MetadataField]> {
        match &self.inner {
            MetadataValueInner::Struct(fields) => Some(fields),
            _ => None,
        }
    }

    /// Returns an object reference, or `None` for another type.
    #[must_use]
    pub const fn as_reference(&self) -> Option<ObjectRef> {
        match self.inner {
            MetadataValueInner::Reference(value) => Some(value),
            _ => None,
        }
    }

    /// Returns this value's nesting depth, including the root value.
    #[must_use]
    pub const fn nesting_depth(&self) -> usize {
        self.depth
    }

    /// Returns the bounded aggregate payload size used for validation.
    #[must_use]
    pub const fn payload_bytes(&self) -> usize {
        self.payload_bytes
    }

    const fn leaf(inner: MetadataValueInner, payload_bytes: usize) -> Self {
        Self {
            inner,
            depth: 1,
            payload_bytes,
        }
    }
}

/// One named member of an ordered structured metadata value.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct MetadataField {
    name: PropertyId,
    value: MetadataValue,
}

impl MetadataField {
    /// Creates a structured metadata field.
    #[must_use]
    pub const fn new(name: PropertyId, value: MetadataValue) -> Self {
        Self { name, value }
    }

    /// Returns the vocabulary-local field name.
    #[must_use]
    pub const fn name(&self) -> &PropertyId {
        &self.name
    }

    /// Returns the field value.
    #[must_use]
    pub const fn value(&self) -> &MetadataValue {
        &self.value
    }
}

/// One independently repeatable assertion about a target object.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct MetadataAssertion {
    property: MetadataProperty,
    value: MetadataValue,
}

impl MetadataAssertion {
    /// Creates an assertion. Repetition is represented by multiple assertions.
    #[must_use]
    pub const fn new(property: MetadataProperty, value: MetadataValue) -> Self {
        Self { property, value }
    }

    /// Returns the asserted property.
    #[must_use]
    pub const fn property(&self) -> &MetadataProperty {
        &self.property
    }

    /// Returns the asserted value.
    #[must_use]
    pub const fn value(&self) -> &MetadataValue {
        &self.value
    }
}

fn validate_identifier(label: &str, value: &str, maximum: usize) -> Result<()> {
    if value.is_empty() || value.len() > maximum {
        return Err(Error::new(
            ErrorKind::InvalidArgument,
            format!("{label} must contain 1-{maximum} UTF-8 bytes"),
        ));
    }
    if value.chars().any(char::is_whitespace) || value.chars().any(char::is_control) {
        return Err(Error::new(
            ErrorKind::InvalidArgument,
            format!("{label} must not contain whitespace or control characters"),
        ));
    }
    Ok(())
}

fn validate_text(label: &str, value: &str, maximum: usize) -> Result<()> {
    if value.len() > maximum || value.contains('\0') {
        return Err(Error::new(
            ErrorKind::InvalidArgument,
            format!("{label} must contain at most {maximum} UTF-8 bytes without NUL"),
        ));
    }
    Ok(())
}

fn validate_language_tag(value: &str) -> Result<()> {
    if value.is_empty()
        || value.len() > MAX_LANGUAGE_TAG_BYTES
        || !value.is_ascii()
        || value.starts_with('-')
        || value.ends_with('-')
        || value
            .bytes()
            .any(|byte| !byte.is_ascii_alphanumeric() && byte != b'-')
    {
        return Err(Error::new(
            ErrorKind::InvalidArgument,
            "metadata language tag must be 1-64 ASCII letters, digits, or separated '-' characters",
        ));
    }
    if value.split('-').any(str::is_empty) {
        return Err(Error::new(
            ErrorKind::InvalidArgument,
            "metadata language tag must not contain empty subtags",
        ));
    }
    Ok(())
}

fn composite_depth(child_depths: impl Iterator<Item = usize>) -> Result<usize> {
    let depth = child_depths.max().unwrap_or(0).saturating_add(1);
    if depth > MAX_METADATA_NESTING_DEPTH {
        return Err(limit_error(
            "metadata value nesting depth",
            MAX_METADATA_NESTING_DEPTH,
        ));
    }
    Ok(depth)
}

fn checked_payload_sum(values: impl IntoIterator<Item = usize>) -> Result<usize> {
    let mut total = 0_usize;
    for value in values {
        total = total
            .checked_add(value)
            .ok_or_else(|| limit_error("metadata aggregate payload", MAX_METADATA_TOTAL_BYTES))?;
        if total > MAX_METADATA_TOTAL_BYTES {
            return Err(limit_error(
                "metadata aggregate payload",
                MAX_METADATA_TOTAL_BYTES,
            ));
        }
    }
    Ok(total)
}

fn limit_error(label: &str, maximum: usize) -> Error {
    Error::new(
        ErrorKind::InvalidArgument,
        format!("{label} exceeds the supported limit of {maximum}"),
    )
}

const fn greatest_common_divisor(mut left: u64, mut right: u64) -> u64 {
    while right != 0 {
        let remainder = left % right;
        left = right;
        right = remainder;
    }
    left
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AssetId, RepresentationId};

    fn property(name: &str) -> MetadataProperty {
        MetadataProperty::new(
            VocabularyId::new("https://example.com/vocabulary").expect("valid vocabulary"),
            PropertyId::new(name).expect("valid property"),
        )
    }

    #[test]
    fn unknown_vocabulary_and_property_round_trip_exactly() {
        let property = MetadataProperty::new(
            VocabularyId::new("com.Example.Custom/1").expect("valid vocabulary"),
            PropertyId::new("CameraSerialNumber").expect("valid property"),
        );

        assert_eq!(property.vocabulary().as_str(), "com.Example.Custom/1");
        assert_eq!(property.property().as_str(), "CameraSerialNumber");
    }

    #[test]
    fn primitive_values_are_typed_and_exact() {
        let string = MetadataValue::string("Interview A").expect("valid string");
        let lang = MetadataValue::language_string("Colour", "en-GB").expect("valid language");
        let uri = MetadataValue::uri("urn:example:Asset%201").expect("valid URI");
        let bytes = MetadataValue::bytes(vec![0, 1, 255]).expect("valid bytes");
        let decimal = MetadataValue::decimal(DecimalValue::new(12_340, 3).unwrap());
        let rational = MetadataValue::rational(RationalValue::new(48_000, 2_000).unwrap());

        assert_eq!(string.kind(), MetadataValueKind::String);
        assert_eq!(string.as_string(), Some("Interview A"));
        assert_eq!(lang.as_language_string(), Some(("Colour", "en-GB")));
        assert_eq!(uri.as_uri(), Some("urn:example:Asset%201"));
        assert_eq!(bytes.as_bytes(), Some(&[0, 1, 255][..]));
        assert_eq!(
            decimal.as_decimal(),
            Some(DecimalValue::new(1_234, 2).unwrap())
        );
        assert_eq!(
            rational.as_rational(),
            Some(RationalValue::new(24, 1).unwrap())
        );
        assert_eq!(MetadataValue::i64(-4).as_i64(), Some(-4));
        assert_eq!(MetadataValue::u64(4).as_u64(), Some(4));
        assert_eq!(MetadataValue::boolean(true).as_bool(), Some(true));
        assert_eq!(
            MetadataValue::timestamp(Timestamp::from_unix_micros(42)).as_timestamp(),
            Some(Timestamp::from_unix_micros(42))
        );
    }

    #[test]
    fn repeated_assertions_are_distinct_from_a_list_value() {
        let assertions = [
            MetadataAssertion::new(property("keyword"), MetadataValue::string("one").unwrap()),
            MetadataAssertion::new(property("keyword"), MetadataValue::string("two").unwrap()),
        ];
        let list = MetadataValue::list(vec![
            MetadataValue::string("one").unwrap(),
            MetadataValue::string("two").unwrap(),
        ])
        .unwrap();

        assert_eq!(assertions.len(), 2);
        assert_eq!(list.as_list().expect("list").len(), 2);
        assert_ne!(assertions[0].value(), &list);
    }

    #[test]
    fn structured_values_preserve_field_order_and_references() {
        let asset = ObjectRef::Asset(AssetId::from_bytes([7; 16]));
        let representation = ObjectRef::Representation(RepresentationId::from_bytes([8; 16]));
        let value = MetadataValue::structure(vec![
            MetadataField::new(
                PropertyId::new("asset").unwrap(),
                MetadataValue::reference(asset),
            ),
            MetadataField::new(
                PropertyId::new("representation").unwrap(),
                MetadataValue::reference(representation),
            ),
        ])
        .expect("valid structure");

        let fields = value.as_structure().expect("structure");
        assert_eq!(fields[0].name().as_str(), "asset");
        assert_eq!(fields[0].value().as_reference(), Some(asset));
        assert_eq!(fields[1].name().as_str(), "representation");
        assert_eq!(fields[1].value().as_reference(), Some(representation));
    }

    #[test]
    fn invalid_identifiers_language_tags_and_uris_are_rejected() {
        assert_eq!(
            VocabularyId::new("contains whitespace")
                .expect_err("invalid vocabulary")
                .kind(),
            ErrorKind::InvalidArgument
        );
        assert_eq!(
            PropertyId::new("").expect_err("empty property").kind(),
            ErrorKind::InvalidArgument
        );
        for language in ["", "-en", "en--GB", "en_uk", "dé"] {
            assert_eq!(
                MetadataValue::language_string("value", language)
                    .expect_err("invalid language")
                    .kind(),
                ErrorKind::InvalidArgument
            );
        }
        assert_eq!(
            MetadataValue::uri("not a uri")
                .expect_err("invalid URI")
                .kind(),
            ErrorKind::InvalidArgument
        );
    }

    #[test]
    fn recursive_values_enforce_depth_and_payload_limits() {
        let mut value = MetadataValue::string("leaf").unwrap();
        for _ in 1..MAX_METADATA_NESTING_DEPTH {
            value = MetadataValue::list(vec![value]).expect("depth remains valid");
        }
        assert_eq!(value.nesting_depth(), MAX_METADATA_NESTING_DEPTH);
        assert_eq!(
            MetadataValue::list(vec![value])
                .expect_err("excess depth must fail")
                .kind(),
            ErrorKind::InvalidArgument
        );

        let chunk = MetadataValue::bytes(vec![0; MAX_METADATA_TOTAL_BYTES / 2 + 1]).unwrap();
        assert_eq!(
            MetadataValue::list(vec![chunk.clone(), chunk])
                .expect_err("excess aggregate payload must fail")
                .kind(),
            ErrorKind::InvalidArgument
        );
    }

    #[test]
    fn rational_normalization_handles_signed_boundaries() {
        assert_eq!(
            RationalValue::new(i64::MIN, 1_u64 << 63).unwrap(),
            RationalValue::new(-1, 1).unwrap()
        );
        assert_eq!(
            RationalValue::new(1, 0)
                .expect_err("zero denominator")
                .kind(),
            ErrorKind::InvalidArgument
        );
    }

    #[test]
    fn decimal_scale_is_bounded_after_normalization() {
        assert_eq!(
            DecimalValue::new(1, MAX_METADATA_DECIMAL_SCALE + 1)
                .expect_err("oversized scale")
                .kind(),
            ErrorKind::InvalidArgument
        );
        assert_eq!(
            DecimalValue::new(10, MAX_METADATA_DECIMAL_SCALE + 1).unwrap(),
            DecimalValue::new(1, MAX_METADATA_DECIMAL_SCALE).unwrap()
        );
    }
}
