//! Structure and membership values for compound representations.

use crate::{Error, ErrorKind, ResourceId, Result};

/// Maximum encoded length of an extensible resource-role identifier.
pub const MAX_RESOURCE_ROLE_BYTES: usize = 128;
/// Maximum combined UTF-8 length of an image-sequence prefix and suffix.
pub const MAX_SEQUENCE_PATTERN_BYTES: usize = 1_024;
/// Maximum supported zero-padding width for an image-sequence frame number.
pub const MAX_FRAME_PADDING: u8 = 32;

/// An inclusive, regularly stepped frame domain.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct FrameRange {
    start: i64,
    end: i64,
    step: u32,
}

impl FrameRange {
    /// Creates an inclusive range whose end is aligned to its step.
    ///
    /// # Errors
    ///
    /// Returns [`ErrorKind::InvalidArgument`] for a reversed range, zero step,
    /// or an end frame not reachable from the start by whole steps.
    pub fn new(start: i64, end: i64, step: u32) -> Result<Self> {
        if step == 0 || end < start {
            return Err(Error::new(
                ErrorKind::InvalidArgument,
                "frame range must be ascending with a non-zero step",
            ));
        }
        let distance = i128::from(end) - i128::from(start);
        if distance % i128::from(step) != 0 {
            return Err(Error::new(
                ErrorKind::InvalidArgument,
                "frame range end must be aligned to its step",
            ));
        }
        Ok(Self { start, end, step })
    }

    /// Returns the first frame number.
    #[must_use]
    pub const fn start(self) -> i64 {
        self.start
    }

    /// Returns the last frame number.
    #[must_use]
    pub const fn end(self) -> i64 {
        self.end
    }

    /// Returns the positive frame-number increment.
    #[must_use]
    pub const fn step(self) -> u32 {
        self.step
    }

    /// Returns the exact number of frames in the regular domain.
    #[must_use]
    pub fn frame_count(self) -> u128 {
        let distance = i128::from(self.end) - i128::from(self.start);
        distance.unsigned_abs() / u128::from(self.step) + 1
    }

    /// Returns whether the frame belongs to the stepped domain.
    #[must_use]
    pub fn contains(self, frame: i64) -> bool {
        if frame < self.start || frame > self.end {
            return false;
        }
        let distance = i128::from(frame) - i128::from(self.start);
        distance % i128::from(self.step) == 0
    }
}

/// The filename components surrounding an image-sequence frame number.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct ImageSequencePattern {
    prefix: String,
    suffix: String,
    padding: u8,
}

impl ImageSequencePattern {
    /// Creates a bounded filename pattern without directory separators.
    ///
    /// # Errors
    ///
    /// Returns [`ErrorKind::InvalidArgument`] when the pattern is empty, too
    /// long, contains a path separator or NUL, or requests excessive padding.
    pub fn new(prefix: impl Into<String>, suffix: impl Into<String>, padding: u8) -> Result<Self> {
        let prefix = prefix.into();
        let suffix = suffix.into();
        let invalid_character = |character| matches!(character, '/' | '\\' | '\0');
        if (prefix.is_empty() && suffix.is_empty())
            || prefix.len().saturating_add(suffix.len()) > MAX_SEQUENCE_PATTERN_BYTES
            || prefix.chars().any(invalid_character)
            || suffix.chars().any(invalid_character)
            || padding > MAX_FRAME_PADDING
        {
            return Err(Error::new(
                ErrorKind::InvalidArgument,
                "invalid image-sequence filename pattern",
            ));
        }
        Ok(Self {
            prefix,
            suffix,
            padding,
        })
    }

    /// Returns the text before the frame number.
    #[must_use]
    pub fn prefix(&self) -> &str {
        &self.prefix
    }

    /// Returns the text after the frame number.
    #[must_use]
    pub fn suffix(&self) -> &str {
        &self.suffix
    }

    /// Returns the minimum frame-number width.
    #[must_use]
    pub const fn padding(&self) -> u8 {
        self.padding
    }

    /// Formats a filename for `frame` without joining it to a locator.
    #[must_use]
    pub fn filename(&self, frame: i64) -> String {
        let frame = format!("{frame:0width$}", width = usize::from(self.padding));
        format!("{}{frame}{}", self.prefix, self.suffix)
    }
}

/// A namespaced, open-world role for a resource within a representation.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ResourceRole(String);

impl ResourceRole {
    /// Creates a role such as `postproject:essence` or `vendor:playlist`.
    ///
    /// # Errors
    ///
    /// Returns [`ErrorKind::InvalidArgument`] when the role is empty, too long,
    /// is not namespaced, or contains unsupported bytes.
    pub fn new(value: impl Into<String>) -> Result<Self> {
        let value = value.into();
        let valid_bytes = value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-' | b':'));
        let valid_namespace = value
            .split_once(':')
            .is_some_and(|(namespace, local)| !namespace.is_empty() && !local.is_empty());
        if value.len() > MAX_RESOURCE_ROLE_BYTES || !valid_bytes || !valid_namespace {
            return Err(Error::new(
                ErrorKind::InvalidArgument,
                format!(
                    "resource role must be a namespaced identifier of at most {MAX_RESOURCE_ROLE_BYTES} ASCII bytes"
                ),
            ));
        }
        Ok(Self(value))
    }

    /// Returns the exact role identifier.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// One resource's role and requiredness within a content structure.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResourceMember {
    resource_id: ResourceId,
    role: ResourceRole,
    required: bool,
}

impl ResourceMember {
    /// Creates a resource membership.
    #[must_use]
    pub const fn new(resource_id: ResourceId, role: ResourceRole, required: bool) -> Self {
        Self {
            resource_id,
            role,
            required,
        }
    }

    /// Returns the participating resource.
    #[must_use]
    pub const fn resource_id(&self) -> ResourceId {
        self.resource_id
    }

    /// Returns the member's extensible semantic role.
    #[must_use]
    pub const fn role(&self) -> &ResourceRole {
        &self.role
    }

    /// Returns whether availability of this member is required for completeness.
    #[must_use]
    pub const fn is_required(&self) -> bool {
        self.required
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_ranges_are_inclusive_and_stepped() {
        let frames = FrameRange::new(-2, 4, 2).expect("valid range");

        assert_eq!(frames.frame_count(), 4);
        assert!(frames.contains(-2));
        assert!(frames.contains(4));
        assert!(!frames.contains(1));
        assert!(FrameRange::new(1, 0, 1).is_err());
        assert!(FrameRange::new(0, 5, 2).is_err());
    }

    #[test]
    fn sequence_patterns_format_frames_without_paths() {
        let pattern = ImageSequencePattern::new("shot.", ".exr", 4).expect("valid pattern");

        assert_eq!(pattern.filename(12), "shot.0012.exr");
        assert_eq!(pattern.filename(-2), "shot.-002.exr");
        assert!(ImageSequencePattern::new("directory/shot.", ".exr", 4).is_err());
        assert!(ImageSequencePattern::new("", "", 0).is_err());
    }

    #[test]
    fn resource_roles_are_namespaced_and_open_world() {
        let standard = ResourceRole::new("postproject:essence").expect("valid role");
        let vendor = ResourceRole::new("example.camera:playlist-v2").expect("valid role");

        assert_eq!(standard.as_str(), "postproject:essence");
        assert_eq!(vendor.as_str(), "example.camera:playlist-v2");
        assert!(ResourceRole::new("essence").is_err());
        assert!(ResourceRole::new("vendor:").is_err());
        assert!(ResourceRole::new("vendor:side car").is_err());
    }

    #[test]
    fn membership_preserves_role_and_requiredness() {
        let resource_id = ResourceId::new();
        let role = ResourceRole::new("postproject:thumbnail").expect("valid role");
        let member = ResourceMember::new(resource_id, role, false);

        assert_eq!(member.resource_id(), resource_id);
        assert_eq!(member.role().as_str(), "postproject:thumbnail");
        assert!(!member.is_required());
    }
}
