//! Preparation of filesystem-backed domain values before persistence.

use std::{fs, path::Path};

use postproject_core::{
    Asset, AssetId, ContentStructure, Locator, LocatorAvailability, LocatorId, MediaRoot,
    MediaRootId, OriginalMediaImport, Representation, RepresentationId, RepresentationKind,
    Resource, ResourceId, Result, Timestamp,
};

use crate::{canonical_file_uri, fingerprint_file, fingerprint_representation};

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
    let path = path.as_ref();
    let report = fingerprint_file(path)?;
    let uri = canonical_file_uri(path)?;
    let now = Timestamp::now()?;
    let asset = Asset::new(AssetId::new(), now, display_name, import_source);
    let (fingerprint, facts, _) = report.into_parts();
    let resource_id = ResourceId::new();
    let structure = ContentStructure::single_resource(resource_id);
    let resource = Resource::new(resource_id, vec![fingerprint], Some(facts));
    let representation_fingerprint =
        fingerprint_representation(&structure, std::slice::from_ref(&resource))?;
    let representation = Representation::new(
        RepresentationId::new(),
        asset.id(),
        RepresentationKind::Original,
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
    OriginalMediaImport::new(asset, representation, vec![resource], vec![locator])
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
    fn root_must_be_a_directory() {
        let file = NamedTempFile::new().expect("create file");
        assert!(prepare_media_root(file.path(), None, 0).is_err());
    }
}
