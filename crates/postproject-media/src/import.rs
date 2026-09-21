//! Preparation of filesystem-backed domain values before persistence.

use std::{
    fs,
    path::{Path, PathBuf},
};

use postproject_core::{
    Asset, AssetId, ContentStructure, FrameRange, ImageSequenceDescriptor, ImageSequencePattern,
    Locator, LocatorAvailability, LocatorId, MediaRoot, MediaRootId, OriginalMediaImport,
    RationalRate, Representation, RepresentationId, RepresentationImport, RepresentationKind,
    Resource, ResourceId, Result, Timestamp,
};

use crate::{canonical_file_uri, fingerprint_file, fingerprint_representation};

/// Explicit filesystem description of one compact image sequence.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ImageSequenceSource {
    directory: PathBuf,
    pattern: ImageSequencePattern,
    frames: FrameRange,
    rate: RationalRate,
    known_missing_frames: Vec<i64>,
}

impl ImageSequenceSource {
    /// Creates a sequence source without scanning its directory.
    #[must_use]
    pub fn new(
        directory: impl Into<PathBuf>,
        pattern: ImageSequencePattern,
        frames: FrameRange,
        rate: RationalRate,
        known_missing_frames: Vec<i64>,
    ) -> Self {
        Self {
            directory: directory.into(),
            pattern,
            frames,
            rate,
            known_missing_frames,
        }
    }
}

/// Inspects a regular file and prepares a validated original-media import.
///
/// This function does not mutate production state. Persist the returned aggregate
/// inside an explicit production transaction.
///
/// # Errors
///
/// Returns errors from path canonicalization, fingerprint calculation, time
/// capture, or domain validation.
pub fn prepare_original_media(
    path: impl AsRef<Path>,
    display_name: Option<String>,
    import_source: Option<String>,
) -> Result<OriginalMediaImport> {
    let now = Timestamp::now()?;
    let asset = Asset::new(AssetId::new(), now, display_name, import_source);
    let prepared = prepare_single_file_representation_at(
        asset.id(),
        RepresentationKind::Original,
        path.as_ref(),
        now,
    )?;
    let (representation, resources, locators) = prepared.into_parts();
    OriginalMediaImport::new(asset, representation, resources, locators)
}

/// Inspects a regular file and prepares a representation for an existing asset.
///
/// The caller chooses the representation kind. This function does not mutate
/// production state; persist the returned aggregate with a production
/// transaction.
///
/// # Errors
///
/// Returns errors from path canonicalization, fingerprint calculation, time
/// capture, or domain validation.
pub fn prepare_single_file_representation(
    asset_id: AssetId,
    kind: RepresentationKind,
    path: impl AsRef<Path>,
) -> Result<RepresentationImport> {
    prepare_single_file_representation_at(asset_id, kind, path.as_ref(), Timestamp::now()?)
}

fn prepare_single_file_representation_at(
    asset_id: AssetId,
    kind: RepresentationKind,
    path: &Path,
    now: Timestamp,
) -> Result<RepresentationImport> {
    let report = fingerprint_file(path)?;
    let uri = canonical_file_uri(path)?;
    let (fingerprint, facts, _) = report.into_parts();
    let resource_id = ResourceId::new();
    let structure = ContentStructure::single_resource(resource_id);
    let resource = Resource::new(resource_id, vec![fingerprint], Some(facts));
    let representation_fingerprint =
        fingerprint_representation(&structure, std::slice::from_ref(&resource))?;
    let representation = Representation::new(
        RepresentationId::new(),
        asset_id,
        kind,
        structure,
        vec![representation_fingerprint],
    );
    let locator = Locator::new(
        LocatorId::new(),
        resource_id,
        uri,
        Some(now),
        LocatorAvailability::Online,
    )?;
    RepresentationImport::new(representation, vec![resource], vec![locator])
}

/// Prepares a compact image-sequence representation for an existing asset.
///
/// The source is explicit: this verifies only that its locator is a directory
/// and records the caller-supplied frame domain and exceptions. Automatic
/// sequence discovery and filesystem frame inventory are separate operations.
///
/// # Errors
///
/// Returns an I/O error when the directory cannot be inspected, an
/// invalid-argument error when it is not a directory, or domain validation
/// errors for an invalid descriptor.
pub fn prepare_image_sequence_representation(
    asset_id: AssetId,
    kind: RepresentationKind,
    source: &ImageSequenceSource,
) -> Result<RepresentationImport> {
    let metadata = fs::metadata(&source.directory).map_err(|error| {
        postproject_core::Error::new(
            postproject_core::ErrorKind::Io,
            format!(
                "cannot read image-sequence directory {}: {error}",
                source.directory.display()
            ),
        )
    })?;
    if !metadata.is_dir() {
        return Err(postproject_core::Error::new(
            postproject_core::ErrorKind::InvalidArgument,
            format!(
                "image-sequence locator is not a directory: {}",
                source.directory.display()
            ),
        ));
    }
    let resource_id = ResourceId::new();
    let descriptor = ImageSequenceDescriptor::new(
        resource_id,
        source.pattern.clone(),
        source.frames,
        source.rate,
        source.known_missing_frames.clone(),
    )?;
    let representation = Representation::new(
        RepresentationId::new(),
        asset_id,
        kind,
        ContentStructure::image_sequence(descriptor),
        Vec::new(),
    );
    let resource = Resource::new(resource_id, Vec::new(), None);
    let locator = Locator::new(
        LocatorId::new(),
        resource_id,
        canonical_file_uri(&source.directory)?,
        Some(Timestamp::now()?),
        LocatorAvailability::Online,
    )?;
    RepresentationImport::new(representation, vec![resource], vec![locator])
}

/// Validates a directory and prepares a canonical resolver media root.
///
/// If `path` itself is a symbolic link, canonicalization stores the target
/// directory URI. Descendant symlinks remain excluded by resolver policy.
///
/// # Errors
///
/// Returns an I/O error when metadata cannot be read, an invalid-argument error
/// when `path` is not a directory, or errors from path/URI domain validation.
pub fn prepare_media_root(
    path: impl AsRef<Path>,
    label: Option<String>,
    priority: i32,
) -> Result<MediaRoot> {
    let path = path.as_ref();
    let metadata = fs::metadata(path).map_err(|error| {
        postproject_core::Error::new(
            postproject_core::ErrorKind::Io,
            format!(
                "cannot read media-root metadata {}: {error}",
                path.display()
            ),
        )
    })?;
    if !metadata.is_dir() {
        return Err(postproject_core::Error::new(
            postproject_core::ErrorKind::InvalidArgument,
            format!("media root is not a directory: {}", path.display()),
        ));
    }
    MediaRoot::new(
        MediaRootId::new(),
        canonical_file_uri(path)?,
        label,
        priority,
        true,
    )
}

/// Prepares a confirmed online locator for transactional persistence.
///
/// The URI is normalized by the domain constructor. This function does not
/// require the URI to use the `file` scheme because future storage transports may
/// confirm other locator types.
///
/// # Errors
///
/// Returns errors from time capture or URI validation.
pub fn prepare_confirmed_locator(
    resource_id: ResourceId,
    uri: impl Into<String>,
) -> Result<Locator> {
    let now = Timestamp::now()?;
    Locator::new(
        LocatorId::new(),
        resource_id,
        uri,
        Some(now),
        LocatorAvailability::Online,
    )
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use tempfile::NamedTempFile;

    use super::*;

    #[test]
    fn prepares_consistent_original_import() {
        let mut file = NamedTempFile::new().expect("create file");
        file.write_all(b"original media").expect("write file");
        let prepared = prepare_original_media(
            file.path(),
            Some("Camera A".to_owned()),
            Some("unit-test".to_owned()),
        )
        .expect("prepare import");

        assert_eq!(prepared.representation().asset_id(), prepared.asset().id());
        assert_eq!(prepared.representation().fingerprints().len(), 1);
        assert_eq!(prepared.resources().len(), 1);
        assert_eq!(prepared.resources()[0].fingerprints().len(), 1);
        assert_eq!(
            prepared.locators()[0].resource_id(),
            prepared.resources()[0].id()
        );
        assert!(prepared.locators()[0].uri().starts_with("file:"));
    }

    #[test]
    fn prepares_non_original_single_file_representation() {
        let mut file = NamedTempFile::new().expect("create file");
        file.write_all(b"proxy media").expect("write file");
        let asset_id = AssetId::new();

        let prepared =
            prepare_single_file_representation(asset_id, RepresentationKind::Proxy, file.path())
                .expect("prepare proxy");

        assert_eq!(prepared.representation().asset_id(), asset_id);
        assert_eq!(prepared.representation().kind(), RepresentationKind::Proxy);
        assert_eq!(prepared.representation().fingerprints().len(), 1);
        assert_eq!(prepared.resources()[0].fingerprints().len(), 1);
        assert_eq!(prepared.locators().len(), 1);
    }

    #[test]
    fn prepares_compact_image_sequence_without_scanning_frames() {
        let directory = tempfile::tempdir().expect("create directory");
        let source = ImageSequenceSource::new(
            directory.path(),
            ImageSequencePattern::new("shot.", ".exr", 4).expect("valid pattern"),
            FrameRange::new(1_001, 1_100, 1).expect("valid range"),
            RationalRate::new(24_000, 1_001).expect("valid rate"),
            vec![1_027],
        );

        let prepared = prepare_image_sequence_representation(
            AssetId::new(),
            RepresentationKind::Derived,
            &source,
        )
        .expect("prepare sequence");

        let descriptor = prepared
            .representation()
            .content_structure()
            .image_sequence_descriptor()
            .expect("sequence descriptor");
        assert_eq!(descriptor.known_missing_frames(), &[1_027]);
        assert_eq!(prepared.resources().len(), 1);
        assert!(prepared.resources()[0].fingerprints().is_empty());
        assert!(prepared.locators()[0].uri().starts_with("file:"));
    }

    #[test]
    fn root_must_be_a_directory() {
        let file = NamedTempFile::new().expect("create file");
        assert!(prepare_media_root(file.path(), None, 0).is_err());
    }
}
