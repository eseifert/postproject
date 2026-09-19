//! Versioned deterministic encoding for persisted metadata values.

use postproject_core::{
    ActivityId, AssetId, DecimalValue, Error, ErrorKind, MAX_METADATA_BINARY_BYTES,
    MAX_METADATA_COLLECTION_ITEMS, MAX_METADATA_NESTING_DEPTH, MAX_METADATA_TEXT_BYTES,
    MAX_METADATA_URI_BYTES, MAX_PROPERTY_ID_BYTES, MetadataField, MetadataValue, MetadataValueKind,
    ObjectRef, ProjectId, PropertyId, RationalValue, RepresentationId, Result, Timestamp,
};

const MAGIC: &[u8; 4] = b"PPMV";
const FORMAT_VERSION: u8 = 1;
const MAX_ENCODED_BYTES: usize = 16 * 1024 * 1024;

const TAG_STRING: u8 = 0;
const TAG_LANGUAGE_STRING: u8 = 1;
const TAG_I64: u8 = 2;
const TAG_U64: u8 = 3;
const TAG_DECIMAL: u8 = 4;
const TAG_BOOL: u8 = 5;
const TAG_TIMESTAMP: u8 = 6;
const TAG_URI: u8 = 7;
const TAG_BYTES: u8 = 8;
const TAG_RATIONAL: u8 = 9;
const TAG_LIST: u8 = 10;
const TAG_STRUCT: u8 = 11;
const TAG_REFERENCE: u8 = 12;

pub(crate) fn encode(value: &MetadataValue) -> Result<Vec<u8>> {
    let mut writer = Writer::new();
    writer.write(MAGIC)?;
    writer.write_u8(FORMAT_VERSION)?;
    encode_value(&mut writer, value)?;
    Ok(writer.finish())
}

pub(crate) fn decode(encoded: &[u8]) -> Result<MetadataValue> {
    if encoded.len() > MAX_ENCODED_BYTES {
        return Err(malformed("encoded metadata exceeds the size limit"));
    }
    let mut reader = Reader::new(encoded);
    if reader.read_exact(MAGIC.len())? != MAGIC {
        return Err(malformed("metadata encoding magic is invalid"));
    }
    let version = reader.read_u8()?;
    if version != FORMAT_VERSION {
        return Err(malformed(format!(
            "metadata encoding version {version} is unsupported"
        )));
    }
    let value = decode_value(&mut reader, 1)?;
    if !reader.is_finished() {
        return Err(malformed("metadata encoding contains trailing bytes"));
    }
    Ok(value)
}

fn encode_value(writer: &mut Writer, value: &MetadataValue) -> Result<()> {
    match value.kind() {
        MetadataValueKind::String => {
            writer.write_u8(TAG_STRING)?;
            writer.write_string(required(value.as_string(), "string")?)
        }
        MetadataValueKind::LangString => {
            writer.write_u8(TAG_LANGUAGE_STRING)?;
            let (text, language) = required(value.as_language_string(), "language string")?;
            writer.write_string(text)?;
            writer.write_string(language)
        }
        MetadataValueKind::I64 => {
            writer.write_u8(TAG_I64)?;
            writer.write_i64(required(value.as_i64(), "signed integer")?)
        }
        MetadataValueKind::U64 => {
            writer.write_u8(TAG_U64)?;
            writer.write_u64(required(value.as_u64(), "unsigned integer")?)
        }
        MetadataValueKind::Decimal => {
            writer.write_u8(TAG_DECIMAL)?;
            let decimal = required(value.as_decimal(), "decimal")?;
            writer.write_i128(decimal.coefficient())?;
            writer.write_u32(decimal.scale())
        }
        MetadataValueKind::Bool => {
            writer.write_u8(TAG_BOOL)?;
            writer.write_u8(u8::from(required(value.as_bool(), "boolean")?))
        }
        MetadataValueKind::Timestamp => {
            writer.write_u8(TAG_TIMESTAMP)?;
            writer.write_i64(required(value.as_timestamp(), "timestamp")?.as_unix_micros())
        }
        MetadataValueKind::Uri => {
            writer.write_u8(TAG_URI)?;
            writer.write_string(required(value.as_uri(), "URI")?)
        }
        MetadataValueKind::Bytes => {
            writer.write_u8(TAG_BYTES)?;
            writer.write_length_prefixed(required(value.as_bytes(), "binary")?)
        }
        MetadataValueKind::Rational => {
            writer.write_u8(TAG_RATIONAL)?;
            let rational = required(value.as_rational(), "rational")?;
            writer.write_i64(rational.numerator())?;
            writer.write_u64(rational.denominator())
        }
        MetadataValueKind::List => {
            writer.write_u8(TAG_LIST)?;
            let values = required(value.as_list(), "list")?;
            writer.write_len(values.len())?;
            for value in values {
                encode_value(writer, value)?;
            }
            Ok(())
        }
        MetadataValueKind::Struct => {
            writer.write_u8(TAG_STRUCT)?;
            let fields = required(value.as_structure(), "structure")?;
            writer.write_len(fields.len())?;
            for field in fields {
                writer.write_string(field.name().as_str())?;
                encode_value(writer, field.value())?;
            }
            Ok(())
        }
        MetadataValueKind::Reference => {
            writer.write_u8(TAG_REFERENCE)?;
            encode_reference(writer, required(value.as_reference(), "reference")?)
        }
        _ => Err(Error::new(
            ErrorKind::Unsupported,
            "metadata value kind is not supported by this encoding version",
        )),
    }
}

fn encode_reference(writer: &mut Writer, reference: ObjectRef) -> Result<()> {
    let (kind, id) = match reference {
        ObjectRef::Project(id) => (0, id.into_bytes()),
        ObjectRef::Asset(id) => (1, id.into_bytes()),
        ObjectRef::Representation(id) => (2, id.into_bytes()),
        ObjectRef::Activity(id) => (3, id.into_bytes()),
        _ => {
            return Err(Error::new(
                ErrorKind::Unsupported,
                "metadata object reference is not supported by this encoding version",
            ));
        }
    };
    writer.write_u8(kind)?;
    writer.write(&id)
}

fn decode_value(reader: &mut Reader<'_>, depth: usize) -> Result<MetadataValue> {
    if depth > MAX_METADATA_NESTING_DEPTH {
        return Err(malformed("metadata value nesting exceeds the limit"));
    }
    match reader.read_u8()? {
        TAG_STRING => MetadataValue::string(reader.read_string(MAX_METADATA_TEXT_BYTES)?),
        TAG_LANGUAGE_STRING => MetadataValue::language_string(
            reader.read_string(MAX_METADATA_TEXT_BYTES)?,
            reader.read_string(postproject_core::MAX_LANGUAGE_TAG_BYTES)?,
        ),
        TAG_I64 => Ok(MetadataValue::i64(reader.read_i64()?)),
        TAG_U64 => Ok(MetadataValue::u64(reader.read_u64()?)),
        TAG_DECIMAL => Ok(MetadataValue::decimal(
            DecimalValue::new(reader.read_i128()?, reader.read_u32()?)
                .map_err(invalid_stored_value)?,
        )),
        TAG_BOOL => match reader.read_u8()? {
            0 => Ok(MetadataValue::boolean(false)),
            1 => Ok(MetadataValue::boolean(true)),
            value => Err(malformed(format!(
                "metadata boolean uses invalid value {value}"
            ))),
        },
        TAG_TIMESTAMP => Ok(MetadataValue::timestamp(Timestamp::from_unix_micros(
            reader.read_i64()?,
        ))),
        TAG_URI => MetadataValue::uri(reader.read_string(MAX_METADATA_URI_BYTES)?),
        TAG_BYTES => MetadataValue::bytes(reader.read_bytes(MAX_METADATA_BINARY_BYTES)?.to_vec()),
        TAG_RATIONAL => Ok(MetadataValue::rational(
            RationalValue::new(reader.read_i64()?, reader.read_u64()?)
                .map_err(invalid_stored_value)?,
        )),
        TAG_LIST => {
            let count = reader.read_count()?;
            let mut values = Vec::with_capacity(count);
            for _ in 0..count {
                values.push(decode_value(reader, depth + 1)?);
            }
            MetadataValue::list(values)
        }
        TAG_STRUCT => {
            let count = reader.read_count()?;
            let mut fields = Vec::with_capacity(count);
            for _ in 0..count {
                let name = PropertyId::new(reader.read_string(MAX_PROPERTY_ID_BYTES)?)
                    .map_err(invalid_stored_value)?;
                fields.push(MetadataField::new(name, decode_value(reader, depth + 1)?));
            }
            MetadataValue::structure(fields)
        }
        TAG_REFERENCE => decode_reference(reader).map(MetadataValue::reference),
        tag => Err(malformed(format!(
            "metadata encoding contains unknown value tag {tag}"
        ))),
    }
    .map_err(invalid_stored_value)
}

fn decode_reference(reader: &mut Reader<'_>) -> Result<ObjectRef> {
    let kind = reader.read_u8()?;
    let bytes: [u8; 16] = reader
        .read_exact(16)?
        .try_into()
        .map_err(|_| malformed("metadata reference UUID is invalid"))?;
    match kind {
        0 => Ok(ObjectRef::Project(ProjectId::from_bytes(bytes))),
        1 => Ok(ObjectRef::Asset(AssetId::from_bytes(bytes))),
        2 => Ok(ObjectRef::Representation(RepresentationId::from_bytes(
            bytes,
        ))),
        3 => Ok(ObjectRef::Activity(ActivityId::from_bytes(bytes))),
        _ => Err(malformed(format!(
            "metadata reference uses unknown object kind {kind}"
        ))),
    }
}

fn required<T>(value: Option<T>, label: &str) -> Result<T> {
    value.ok_or_else(|| {
        Error::new(
            ErrorKind::Internal,
            format!("metadata {label} kind has no matching value"),
        )
    })
}

fn invalid_stored_value(error: Error) -> Error {
    if error.kind() == ErrorKind::Storage {
        error
    } else {
        malformed(format!("stored metadata value is invalid: {error}"))
    }
}

fn malformed(message: impl Into<String>) -> Error {
    Error::new(ErrorKind::Storage, message)
}

struct Writer {
    bytes: Vec<u8>,
}

impl Writer {
    fn new() -> Self {
        Self { bytes: Vec::new() }
    }

    fn finish(self) -> Vec<u8> {
        self.bytes
    }

    fn write(&mut self, bytes: &[u8]) -> Result<()> {
        let new_len = self
            .bytes
            .len()
            .checked_add(bytes.len())
            .ok_or_else(|| malformed("encoded metadata size overflow"))?;
        if new_len > MAX_ENCODED_BYTES {
            return Err(Error::new(
                ErrorKind::InvalidArgument,
                "encoded metadata exceeds the size limit",
            ));
        }
        self.bytes.extend_from_slice(bytes);
        Ok(())
    }

    fn write_u8(&mut self, value: u8) -> Result<()> {
        self.write(&[value])
    }

    fn write_u32(&mut self, value: u32) -> Result<()> {
        self.write(&value.to_be_bytes())
    }

    fn write_i64(&mut self, value: i64) -> Result<()> {
        self.write(&value.to_be_bytes())
    }

    fn write_u64(&mut self, value: u64) -> Result<()> {
        self.write(&value.to_be_bytes())
    }

    fn write_i128(&mut self, value: i128) -> Result<()> {
        self.write(&value.to_be_bytes())
    }

    fn write_len(&mut self, value: usize) -> Result<()> {
        let value = u32::try_from(value).map_err(|_| {
            Error::new(
                ErrorKind::InvalidArgument,
                "metadata collection length cannot be encoded",
            )
        })?;
        self.write_u32(value)
    }

    fn write_length_prefixed(&mut self, bytes: &[u8]) -> Result<()> {
        self.write_len(bytes.len())?;
        self.write(bytes)
    }

    fn write_string(&mut self, value: &str) -> Result<()> {
        self.write_length_prefixed(value.as_bytes())
    }
}

struct Reader<'encoded> {
    encoded: &'encoded [u8],
    position: usize,
}

impl<'encoded> Reader<'encoded> {
    const fn new(encoded: &'encoded [u8]) -> Self {
        Self {
            encoded,
            position: 0,
        }
    }

    fn is_finished(&self) -> bool {
        self.position == self.encoded.len()
    }

    fn read_exact(&mut self, length: usize) -> Result<&'encoded [u8]> {
        let end = self
            .position
            .checked_add(length)
            .ok_or_else(|| malformed("metadata encoding offset overflow"))?;
        let value = self
            .encoded
            .get(self.position..end)
            .ok_or_else(|| malformed("metadata encoding is truncated"))?;
        self.position = end;
        Ok(value)
    }

    fn read_u8(&mut self) -> Result<u8> {
        Ok(self.read_exact(1)?[0])
    }

    fn read_u32(&mut self) -> Result<u32> {
        let bytes = self.read_exact(4)?;
        Ok(u32::from_be_bytes(
            bytes
                .try_into()
                .map_err(|_| malformed("metadata u32 is truncated"))?,
        ))
    }

    fn read_i64(&mut self) -> Result<i64> {
        let bytes = self.read_exact(8)?;
        Ok(i64::from_be_bytes(
            bytes
                .try_into()
                .map_err(|_| malformed("metadata i64 is truncated"))?,
        ))
    }

    fn read_u64(&mut self) -> Result<u64> {
        let bytes = self.read_exact(8)?;
        Ok(u64::from_be_bytes(
            bytes
                .try_into()
                .map_err(|_| malformed("metadata u64 is truncated"))?,
        ))
    }

    fn read_i128(&mut self) -> Result<i128> {
        let bytes = self.read_exact(16)?;
        Ok(i128::from_be_bytes(
            bytes
                .try_into()
                .map_err(|_| malformed("metadata i128 is truncated"))?,
        ))
    }

    fn read_len(&mut self) -> Result<usize> {
        usize::try_from(self.read_u32()?)
            .map_err(|_| malformed("metadata length cannot be represented on this platform"))
    }

    fn read_bytes(&mut self, maximum: usize) -> Result<&'encoded [u8]> {
        let length = self.read_len()?;
        if length > maximum {
            return Err(malformed(format!(
                "metadata byte length {length} exceeds limit {maximum}"
            )));
        }
        self.read_exact(length)
    }

    fn read_string(&mut self, maximum: usize) -> Result<String> {
        let bytes = self.read_bytes(maximum)?;
        std::str::from_utf8(bytes)
            .map(str::to_owned)
            .map_err(|error| malformed(format!("metadata text is not UTF-8: {error}")))
    }

    fn read_count(&mut self) -> Result<usize> {
        let count = self.read_len()?;
        if count > MAX_METADATA_COLLECTION_ITEMS {
            return Err(malformed(format!(
                "metadata collection count {count} exceeds limit {MAX_METADATA_COLLECTION_ITEMS}"
            )));
        }
        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn round_trip(value: &MetadataValue) {
        let first = encode(value).expect("encode metadata");
        let second = encode(value).expect("encode metadata again");
        assert_eq!(first, second, "encoding must be deterministic");
        assert_eq!(&decode(&first).expect("decode metadata"), value);
    }

    #[test]
    fn every_value_kind_round_trips_deterministically() {
        let values = vec![
            MetadataValue::string("title").unwrap(),
            MetadataValue::language_string("Farbe", "de-DE").unwrap(),
            MetadataValue::i64(i64::MIN),
            MetadataValue::u64(u64::MAX),
            MetadataValue::decimal(DecimalValue::new(-12_345, 3).unwrap()),
            MetadataValue::boolean(true),
            MetadataValue::timestamp(Timestamp::from_unix_micros(-42)),
            MetadataValue::uri("urn:example:media:1").unwrap(),
            MetadataValue::bytes(vec![0, 1, 2, 255]).unwrap(),
            MetadataValue::rational(RationalValue::new(-24_000, 1_001).unwrap()),
            MetadataValue::reference(ObjectRef::Project(ProjectId::from_bytes([1; 16]))),
            MetadataValue::reference(ObjectRef::Asset(AssetId::from_bytes([2; 16]))),
            MetadataValue::reference(ObjectRef::Representation(RepresentationId::from_bytes(
                [3; 16],
            ))),
            MetadataValue::reference(ObjectRef::Activity(ActivityId::from_bytes([4; 16]))),
        ];
        for value in values {
            round_trip(&value);
        }
    }

    #[test]
    fn nested_ordered_values_round_trip() {
        let value = MetadataValue::structure(vec![
            MetadataField::new(
                PropertyId::new("keywords").unwrap(),
                MetadataValue::list(vec![
                    MetadataValue::string("first").unwrap(),
                    MetadataValue::string("second").unwrap(),
                ])
                .unwrap(),
            ),
            MetadataField::new(
                PropertyId::new("empty").unwrap(),
                MetadataValue::structure(Vec::new()).unwrap(),
            ),
        ])
        .unwrap();

        round_trip(&value);
    }

    #[test]
    fn malformed_encodings_fail_as_storage_errors() {
        let valid = encode(&MetadataValue::string("value").unwrap()).unwrap();
        let mut cases = vec![
            Vec::new(),
            b"NOPE\x01\x00\x00\x00\x00\x00".to_vec(),
            b"PPMV\x02\x00\x00\x00\x00\x00".to_vec(),
            b"PPMV\x01\xff".to_vec(),
            b"PPMV\x01\x05\x02".to_vec(),
            b"PPMV\x01\x0c\xff\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00"
                .to_vec(),
            b"PPMV\x01\x00\x00\x00\x00\x01\xff".to_vec(),
        ];
        let mut trailing = valid.clone();
        trailing.push(0);
        cases.push(trailing);
        cases.push(valid[..valid.len() - 1].to_vec());

        for encoded in cases {
            assert_eq!(
                decode(&encoded)
                    .expect_err("malformed encoding must fail")
                    .kind(),
                ErrorKind::Storage
            );
        }
    }

    #[test]
    fn decoder_rejects_excessive_depth_before_recursing_further() {
        let mut encoded = Vec::from(*MAGIC);
        encoded.push(FORMAT_VERSION);
        for _ in 0..MAX_METADATA_NESTING_DEPTH {
            encoded.push(TAG_LIST);
            encoded.extend_from_slice(&1_u32.to_be_bytes());
        }
        encoded.push(TAG_STRING);
        encoded.extend_from_slice(&0_u32.to_be_bytes());

        assert_eq!(
            decode(&encoded).expect_err("excess depth must fail").kind(),
            ErrorKind::Storage
        );
    }
}
