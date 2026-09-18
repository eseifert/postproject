//! Preparation of filesystem-backed domain values before persistence.

use std::{fs, path::Path};

use postproject_core::{
    Asset, AssetId, Location, LocationAvailability, LocationId, MediaRoot, MediaRootId,
    OriginalMediaImport, Representation, RepresentationId, RepresentationKind, Result, Timestamp,
};

use crate::{canonical_file_uri, fingerprint_file};

/// Inspects a regular file and prepares a validated original-media import.
///
/// This function does not mutate project state. Persist the returned aggregate
/// inside an explicit project transaction.
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
    let path = path.as_ref();
    let report = fingerprint_file(path)?;
    let uri = canonical_file_uri(path)?;
    let now = Timestamp::now()?;
    let asset = Asset::new(AssetId::new(), now, display_name, import_source);
    let (fingerprint, facts, _) = report.into_parts();
    let representation = Representation::new(
        RepresentationId::new(),
        asset.id(),
        RepresentationKind::Original,
        Some(fingerprint),
        Some(facts),
    );
    let location = Location::new(
        LocationId::new(),
        representation.id(),
        uri,
        Some(now),
        LocationAvailability::Online,
    )?;
    OriginalMediaImport::new(asset, representation, location)
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

/// Prepares a confirmed online location for transactional persistence.
///
/// The URI is normalized by the domain constructor. This function does not
/// require the URI to use the `file` scheme because future storage transports may
/// confirm other locator types.
///
/// # Errors
///
/// Returns errors from time capture or URI validation.
pub fn prepare_confirmed_location(
    representation_id: RepresentationId,
    uri: impl Into<String>,
) -> Result<Location> {
    let now = Timestamp::now()?;
    Location::new(
        LocationId::new(),
        representation_id,
        uri,
        Some(now),
        LocationAvailability::Online,
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
        assert_eq!(
            prepared.location().representation_id(),
            prepared.representation().id()
        );
        assert!(prepared.representation().fingerprint().is_some());
        assert!(prepared.location().uri().starts_with("file:"));
    }

    #[test]
    fn root_must_be_a_directory() {
        let file = NamedTempFile::new().expect("create file");
        assert!(prepare_media_root(file.path(), None, 0).is_err());
    }
}
