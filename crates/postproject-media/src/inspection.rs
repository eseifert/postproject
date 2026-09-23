//! Bounded technical-media inspection through an `ffprobe` subprocess.

use std::{
    collections::BTreeMap,
    fs,
    io::Read,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

use postproject_core::{
    DecimalValue, Error, ErrorKind, MetadataAssertion, MetadataField, MetadataProperty,
    MetadataValue, PropertyId, RationalValue, Result, VocabularyId,
};
use serde::Deserialize;

/// Vocabulary used for normalized technical inspection results.
pub const TECHNICAL_METADATA_VOCABULARY: &str = "https://postproject.org/ns/technical-media/1";
/// Property containing one structured `ffprobe` inspection result.
pub const TECHNICAL_INSPECTION_PROPERTY: &str = "inspection";
const MAX_OUTPUT_BYTES: usize = 8 * 1024 * 1024;
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);

/// Result of an optional inspection capability.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum InspectionOutcome {
    /// Inspection succeeded and produced vocabulary-backed metadata.
    Inspected(TechnicalMetadata),
    /// The configured inspector executable is not installed.
    Unavailable {
        /// Human-readable capability diagnostic.
        reason: String,
    },
    /// The inspector ran but did not produce safe, usable output.
    Failed {
        /// Human-readable execution or parsing diagnostic.
        reason: String,
    },
}

/// Validated metadata ready to attach to a representation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TechnicalMetadata {
    assertions: Vec<MetadataAssertion>,
}

impl TechnicalMetadata {
    /// Returns normalized assertions in deterministic order.
    #[must_use]
    pub fn assertions(&self) -> &[MetadataAssertion] {
        &self.assertions
    }
}

/// Adapter boundary for optional technical-media inspection.
pub trait MediaInspector {
    /// Inspects one local media file without mutating production state.
    ///
    /// # Errors
    ///
    /// Returns an error only when the adapter itself cannot safely manage its
    /// subprocess. Missing capability and untrusted tool output are outcomes.
    fn inspect(&self, path: &Path) -> Result<InspectionOutcome>;
}

/// `ffprobe` subprocess implementation of [`MediaInspector`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FfprobeInspector {
    executable: PathBuf,
    timeout: Duration,
}

impl Default for FfprobeInspector {
    fn default() -> Self {
        Self {
            executable: PathBuf::from("ffprobe"),
            timeout: DEFAULT_TIMEOUT,
        }
    }
}

impl FfprobeInspector {
    /// Creates an inspector for a specific executable, primarily for hosts and tests.
    #[must_use]
    pub fn with_executable(executable: impl Into<PathBuf>) -> Self {
        Self {
            executable: executable.into(),
            timeout: DEFAULT_TIMEOUT,
        }
    }

    /// Sets the maximum subprocess duration.
    ///
    /// # Errors
    ///
    /// Returns an invalid-argument error for a zero timeout.
    pub fn with_timeout(mut self, timeout: Duration) -> Result<Self> {
        if timeout.is_zero() {
            return Err(Error::new(
                ErrorKind::InvalidArgument,
                "ffprobe timeout must be greater than zero",
            ));
        }
        self.timeout = timeout;
        Ok(self)
    }
}

impl MediaInspector for FfprobeInspector {
    fn inspect(&self, path: &Path) -> Result<InspectionOutcome> {
        let canonical = fs::canonicalize(path).map_err(|error| {
            Error::new(
                ErrorKind::Io,
                format!("cannot canonicalize media {}: {error}", path.display()),
            )
        })?;
        let mut child = match Command::new(&self.executable)
            .args([
                "-v",
                "error",
                "-print_format",
                "json",
                "-show_format",
                "-show_streams",
            ])
            .arg(canonical)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
        {
            Ok(child) => child,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(InspectionOutcome::Unavailable {
                    reason: format!("{} is not installed", self.executable.display()),
                });
            }
            Err(error) => {
                return Err(Error::new(
                    ErrorKind::Io,
                    format!("cannot start {}: {error}", self.executable.display()),
                ));
            }
        };
        let stdout = child.stdout.take().ok_or_else(|| {
            Error::new(ErrorKind::Internal, "ffprobe stdout pipe was not created")
        })?;
        let stderr = child.stderr.take().ok_or_else(|| {
            Error::new(ErrorKind::Internal, "ffprobe stderr pipe was not created")
        })?;
        let stdout_reader = thread::spawn(move || read_bounded(stdout));
        let stderr_reader = thread::spawn(move || read_bounded(stderr));
        let deadline = Instant::now() + self.timeout;
        let status = loop {
            match child.try_wait() {
                Ok(Some(status)) => break Some(status),
                Ok(None) if Instant::now() < deadline => thread::sleep(Duration::from_millis(10)),
                Ok(None) => {
                    let _ = child.kill();
                    let _ = child.wait();
                    break None;
                }
                Err(error) => {
                    return Err(Error::new(
                        ErrorKind::Io,
                        format!("wait for ffprobe: {error}"),
                    ));
                }
            }
        };
        let stdout = join_reader(stdout_reader)?;
        let stderr = join_reader(stderr_reader)?;
        let Some(status) = status else {
            return Ok(InspectionOutcome::Failed {
                reason: format!(
                    "ffprobe exceeded its {} ms timeout",
                    self.timeout.as_millis()
                ),
            });
        };
        if stdout.exceeded || stderr.exceeded {
            return Ok(InspectionOutcome::Failed {
                reason: format!("ffprobe output exceeded {MAX_OUTPUT_BYTES} bytes"),
            });
        }
        if !status.success() {
            return Ok(InspectionOutcome::Failed {
                reason: bounded_message("ffprobe failed", &stderr.bytes),
            });
        }
        match parse_probe_output(&stdout.bytes) {
            Ok(metadata) => Ok(InspectionOutcome::Inspected(metadata)),
            Err(error) => Ok(InspectionOutcome::Failed {
                reason: error.to_string(),
            }),
        }
    }
}

struct BoundedOutput {
    bytes: Vec<u8>,
    exceeded: bool,
}

fn read_bounded(reader: impl Read) -> std::io::Result<BoundedOutput> {
    let mut bytes = Vec::new();
    let limit = u64::try_from(MAX_OUTPUT_BYTES).unwrap_or(u64::MAX);
    reader.take(limit + 1).read_to_end(&mut bytes)?;
    let exceeded = bytes.len() > MAX_OUTPUT_BYTES;
    if exceeded {
        bytes.truncate(MAX_OUTPUT_BYTES);
    }
    Ok(BoundedOutput { bytes, exceeded })
}

fn join_reader(
    reader: thread::JoinHandle<std::io::Result<BoundedOutput>>,
) -> Result<BoundedOutput> {
    reader
        .join()
        .map_err(|_| Error::new(ErrorKind::Internal, "ffprobe output reader panicked"))?
        .map_err(|error| Error::new(ErrorKind::Io, format!("read ffprobe output: {error}")))
}

fn bounded_message(prefix: &str, bytes: &[u8]) -> String {
    let text = String::from_utf8_lossy(bytes);
    let text = text.trim();
    if text.is_empty() {
        prefix.to_owned()
    } else {
        format!("{prefix}: {}", text.chars().take(1_024).collect::<String>())
    }
}

#[derive(Debug, Deserialize)]
struct ProbeOutput {
    #[serde(default)]
    streams: Vec<ProbeStream>,
    format: Option<ProbeFormat>,
}

#[derive(Debug, Deserialize)]
struct ProbeFormat {
    format_name: Option<String>,
    format_long_name: Option<String>,
    duration: Option<String>,
    size: Option<String>,
    bit_rate: Option<String>,
    #[serde(default)]
    tags: BTreeMap<String, String>,
}

#[derive(Debug, Deserialize)]
struct ProbeStream {
    index: Option<u64>,
    codec_name: Option<String>,
    codec_long_name: Option<String>,
    codec_type: Option<String>,
    width: Option<u64>,
    height: Option<u64>,
    pix_fmt: Option<String>,
    sample_rate: Option<String>,
    channels: Option<u64>,
    channel_layout: Option<String>,
    bits_per_raw_sample: Option<String>,
    avg_frame_rate: Option<String>,
    duration: Option<String>,
    #[serde(default)]
    tags: BTreeMap<String, String>,
}

fn parse_probe_output(bytes: &[u8]) -> Result<TechnicalMetadata> {
    let probe: ProbeOutput = serde_json::from_slice(bytes).map_err(|error| {
        Error::new(
            ErrorKind::InvalidArgument,
            format!("ffprobe returned invalid JSON: {error}"),
        )
    })?;
    let mut fields = Vec::new();
    if let Some(format) = probe.format {
        push_text(&mut fields, "container", format.format_name)?;
        push_text(&mut fields, "container-label", format.format_long_name)?;
        push_numeric_text(&mut fields, "duration-seconds", format.duration)?;
        push_u64_text(&mut fields, "size-bytes", format.size)?;
        push_u64_text(&mut fields, "bit-rate", format.bit_rate)?;
        push_tags(&mut fields, "format-tags", format.tags)?;
    }
    let streams = probe
        .streams
        .into_iter()
        .map(stream_value)
        .collect::<Result<Vec<_>>>()?;
    fields.push(field("streams", MetadataValue::list(streams)?)?);
    let value = MetadataValue::structure(fields)?;
    let property = MetadataProperty::new(
        VocabularyId::new(TECHNICAL_METADATA_VOCABULARY)?,
        PropertyId::new(TECHNICAL_INSPECTION_PROPERTY)?,
    );
    Ok(TechnicalMetadata {
        assertions: vec![MetadataAssertion::new(property, value)],
    })
}

fn stream_value(stream: ProbeStream) -> Result<MetadataValue> {
    let mut fields = Vec::new();
    if let Some(index) = stream.index {
        fields.push(field("index", MetadataValue::u64(index))?);
    }
    push_text(&mut fields, "codec", stream.codec_name)?;
    push_text(&mut fields, "codec-label", stream.codec_long_name)?;
    push_text(&mut fields, "media-type", stream.codec_type)?;
    push_u64(&mut fields, "width", stream.width)?;
    push_u64(&mut fields, "height", stream.height)?;
    push_text(&mut fields, "pixel-format", stream.pix_fmt)?;
    push_u64_text(&mut fields, "sample-rate", stream.sample_rate)?;
    push_u64(&mut fields, "channels", stream.channels)?;
    push_text(&mut fields, "channel-layout", stream.channel_layout)?;
    push_u64_text(&mut fields, "bit-depth", stream.bits_per_raw_sample)?;
    push_rational_text(&mut fields, "average-frame-rate", stream.avg_frame_rate)?;
    push_numeric_text(&mut fields, "duration-seconds", stream.duration)?;
    push_tags(&mut fields, "tags", stream.tags)?;
    MetadataValue::structure(fields)
}

fn field(name: &str, value: MetadataValue) -> Result<MetadataField> {
    Ok(MetadataField::new(PropertyId::new(name)?, value))
}

fn push_text(fields: &mut Vec<MetadataField>, name: &str, value: Option<String>) -> Result<()> {
    if let Some(value) = value {
        fields.push(field(name, MetadataValue::string(value)?)?);
    }
    Ok(())
}

fn push_u64(fields: &mut Vec<MetadataField>, name: &str, value: Option<u64>) -> Result<()> {
    if let Some(value) = value {
        fields.push(field(name, MetadataValue::u64(value))?);
    }
    Ok(())
}

fn push_u64_text(fields: &mut Vec<MetadataField>, name: &str, value: Option<String>) -> Result<()> {
    if let Some(value) = value {
        let value = value
            .parse()
            .map(MetadataValue::u64)
            .unwrap_or(MetadataValue::string(value)?);
        fields.push(field(name, value)?);
    }
    Ok(())
}

fn push_numeric_text(
    fields: &mut Vec<MetadataField>,
    name: &str,
    value: Option<String>,
) -> Result<()> {
    if let Some(value) = value {
        let metadata = decimal_value(&value)
            .map(MetadataValue::decimal)
            .unwrap_or(MetadataValue::string(value)?);
        fields.push(field(name, metadata)?);
    }
    Ok(())
}

fn push_rational_text(
    fields: &mut Vec<MetadataField>,
    name: &str,
    value: Option<String>,
) -> Result<()> {
    if let Some(value) = value {
        let rational = value.split_once('/').and_then(|(numerator, denominator)| {
            RationalValue::new(numerator.parse().ok()?, denominator.parse().ok()?).ok()
        });
        let metadata = rational
            .map(MetadataValue::rational)
            .unwrap_or(MetadataValue::string(value)?);
        fields.push(field(name, metadata)?);
    }
    Ok(())
}

fn push_tags(
    fields: &mut Vec<MetadataField>,
    name: &str,
    tags: BTreeMap<String, String>,
) -> Result<()> {
    if tags.is_empty() {
        return Ok(());
    }
    let values = tags
        .into_iter()
        .map(|(key, value)| {
            MetadataValue::structure(vec![
                field("key", MetadataValue::string(key)?)?,
                field("value", MetadataValue::string(value)?)?,
            ])
        })
        .collect::<Result<Vec<_>>>()?;
    fields.push(field(name, MetadataValue::list(values)?)?);
    Ok(())
}

fn decimal_value(value: &str) -> Option<DecimalValue> {
    let (negative, value) = value
        .strip_prefix('-')
        .map_or((false, value), |value| (true, value));
    let (whole, fraction) = value.split_once('.').unwrap_or((value, ""));
    if whole.is_empty()
        || !whole.bytes().all(|byte| byte.is_ascii_digit())
        || !fraction.bytes().all(|byte| byte.is_ascii_digit())
    {
        return None;
    }
    let digits = format!("{whole}{fraction}");
    let mut coefficient = digits.parse::<i128>().ok()?;
    if negative {
        coefficient = -coefficient;
    }
    DecimalValue::new(coefficient, u32::try_from(fraction.len()).ok()?).ok()
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::*;

    #[test]
    fn bounds_subprocess_output_before_parsing() {
        let output = vec![0_u8; MAX_OUTPUT_BYTES + 1];
        let bounded = read_bounded(Cursor::new(output)).expect("read bounded output");
        assert!(bounded.exceeded);
        assert_eq!(bounded.bytes.len(), MAX_OUTPUT_BYTES);
    }

    #[test]
    fn parses_exact_decimal_values_without_floats() {
        let value = decimal_value("-12.3400").expect("decimal");
        assert_eq!(value.coefficient(), -1234);
        assert_eq!(value.scale(), 2);
    }
}
