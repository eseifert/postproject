//! Deterministic, bounded filesystem media resolution.

use std::{
    collections::{BTreeMap, BTreeSet},
    ffi::OsStr,
    fs,
    path::{Path, PathBuf},
};

use postproject_core::{
    Confidence, ContentStructure, Error, ErrorKind, EvidenceKind, FileFacts,
    ImageSequenceDescriptor, Locator, MAX_SEQUENCE_EXCEPTIONS, MediaRoot, ResolutionCandidate,
    ResolutionEvidence, Resource, ResourceFingerprint, ResourceResolution, ResourceResolutionState,
    Result,
};
use url::Url;
use walkdir::WalkDir;

use crate::{
    FULL_FINGERPRINT_ALGORITHM, SEQUENCE_FINGERPRINT_ALGORITHM, canonical_file_uri,
    fingerprint_file, fingerprint_image_sequence,
};

/// One machine's directory mapping for a production-portable root name.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MediaRootMapping {
    name: String,
    directory: PathBuf,
}

impl MediaRootMapping {
    /// Creates a mapping to an existing local directory.
    ///
    /// # Errors
    ///
    /// Returns a domain error when the name is invalid, the path cannot be
    /// inspected, or the path is not a directory.
    pub fn new(name: impl Into<String>, directory: impl AsRef<Path>) -> Result<Self> {
        let name = name.into();
        MediaRoot::validate_name(&name)?;
        let directory = directory.as_ref();
        let metadata = fs::metadata(directory).map_err(|error| {
            Error::new(
                ErrorKind::Io,
                format!("cannot read mapped root {}: {error}", directory.display()),
            )
        })?;
        if !metadata.is_dir() {
            return Err(Error::new(
                ErrorKind::InvalidArgument,
                format!("mapped root is not a directory: {}", directory.display()),
            ));
        }
        let directory = directory.canonicalize().map_err(|error| {
            Error::new(
                ErrorKind::Io,
                format!(
                    "cannot canonicalize mapped root {}: {error}",
                    directory.display()
                ),
            )
        })?;
        Ok(Self { name, directory })
    }

    /// Returns the logical root name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns this machine's canonical directory.
    #[must_use]
    pub fn directory(&self) -> &Path {
        &self.directory
    }
}

struct Discovery {
    candidates: BTreeMap<String, Vec<ResolutionEvidence>>,
    diagnostics: Vec<ResolutionEvidence>,
}

struct SequenceCandidate {
    candidate: ResolutionCandidate,
    missing_frames: Vec<i64>,
}

struct SearchableRoots {
    directories: Vec<(String, PathBuf)>,
    diagnostics: Vec<ResolutionEvidence>,
}

/// Resource limits applied to one resolver operation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ResolverOptions {
    /// Maximum directory depth, where the root itself has depth zero.
    pub max_depth: usize,
    /// Maximum number of directory entries visited across all roots.
    pub max_entries: usize,
}

impl Default for ResolverOptions {
    fn default() -> Self {
        Self {
            max_depth: 64,
            max_entries: 100_000,
        }
    }
}

/// Resolves unavailable representations under configured filesystem roots.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct MediaResolver {
    options: ResolverOptions,
}

impl MediaResolver {
    /// Creates a resolver after validating its resource limits.
    ///
    /// # Errors
    ///
    /// Returns [`ErrorKind::InvalidArgument`] if either limit is zero.
    pub fn new(options: ResolverOptions) -> Result<Self> {
        if options.max_depth == 0 || options.max_entries == 0 {
            return Err(Error::new(
                ErrorKind::InvalidArgument,
                "resolver depth and entry limits must be greater than zero",
            ));
        }
        Ok(Self { options })
    }

    /// Resolves one resource without mutating production state.
    ///
    /// Known locators are checked before roots. Root traversal does not follow
    /// symlinks, is ordered by filename, stops at configured bounds, filters by
    /// stored size before hashing, and returns all equally credible matches.
    ///
    /// # Errors
    ///
    /// Returns [`ErrorKind::InvalidArgument`] if a known locator belongs to a
    /// different resource, and otherwise only when a result value cannot
    /// be constructed.
    /// Filesystem discovery failures are represented as
    /// [`ResourceResolutionState::Error`] with [`EvidenceKind::DiscoveryError`].
    pub fn resolve_resource(
        &self,
        resource: &Resource,
        structure: &ContentStructure,
        known_locators: &[Locator],
        media_roots: &[MediaRoot],
        root_mappings: &[MediaRootMapping],
    ) -> Result<ResourceResolution> {
        if !structure.resource_ids().contains(&resource.id()) {
            return Err(Error::new(
                ErrorKind::InvalidArgument,
                "resource does not belong to the supplied content structure",
            ));
        }
        if known_locators
            .iter()
            .any(|locator| locator.resource_id() != resource.id())
        {
            return Err(Error::new(
                ErrorKind::InvalidArgument,
                "known locator belongs to a different resource",
            ));
        }
        if let Some(sequence) = structure
            .image_sequence_descriptor()
            .filter(|sequence| sequence.resource_id() == resource.id())
        {
            let (candidate, missing_frames) =
                match online_sequence_candidate(known_locators, sequence) {
                    Ok(Some(result)) => result,
                    Ok(None) => {
                        return self.resolve_moved_sequence(
                            resource,
                            sequence,
                            media_roots,
                            root_mappings,
                        );
                    }
                    Err(detail) => return error_resolution(resource.id(), detail),
                };
            return ResourceResolution::new(
                resource.id(),
                ResourceResolutionState::OnlineAtKnownLocator,
                vec![candidate],
                Vec::new(),
            )?
            .with_missing_frames(missing_frames);
        }
        if let Some(candidate) = online_known_candidate(known_locators)? {
            return ResourceResolution::new(
                resource.id(),
                ResourceResolutionState::OnlineAtKnownLocator,
                vec![candidate],
                Vec::new(),
            );
        }

        let original_name = known_locators.iter().find_map(|locator| {
            file_uri_to_path(locator.uri())
                .ok()
                .and_then(|path| path.file_name().map(OsStr::to_os_string))
        });
        let discovered = match self.discover(
            media_roots,
            root_mappings,
            resource.file_facts(),
            !resource.fingerprints().is_empty(),
            original_name.as_deref(),
        ) {
            Ok(discovered) => discovered,
            Err(detail) => return error_resolution(resource.id(), detail),
        };

        let mut candidates = Vec::new();
        for (uri, cheap_evidence) in discovered.candidates {
            let path = file_uri_to_path(&uri).map_err(|error| {
                Error::new(
                    ErrorKind::Internal,
                    format!("discovered URI cannot be converted back to a path: {error}"),
                )
            })?;
            match verify_candidate(&path, &uri, cheap_evidence, resource.fingerprints()) {
                Ok(Some(candidate)) => candidates.push(candidate),
                Ok(None) => {}
                Err(detail) => return error_resolution(resource.id(), detail),
            }
        }

        let state = match candidates.len() {
            0 if !discovered.diagnostics.is_empty() => ResourceResolutionState::Error,
            0 => ResourceResolutionState::Offline,
            1 if candidates[0].confidence() == Confidence::CERTAIN => {
                ResourceResolutionState::ResolvedExact
            }
            1 => ResourceResolutionState::ResolvedProbable,
            _ => ResourceResolutionState::Ambiguous,
        };
        let mut evidence = discovered.diagnostics;
        if state == ResourceResolutionState::Ambiguous {
            evidence.push(ResolutionEvidence::new(
                EvidenceKind::ConflictingCandidate,
                Some(format!(
                    "{} candidates have matching identity evidence",
                    candidates.len()
                )),
            ));
        }
        ResourceResolution::new(resource.id(), state, candidates, evidence)
    }

    fn resolve_moved_sequence(
        &self,
        resource: &Resource,
        descriptor: &ImageSequenceDescriptor,
        roots: &[MediaRoot],
        mappings: &[MediaRootMapping],
    ) -> Result<ResourceResolution> {
        let (found, mut diagnostics) =
            match self.discover_sequences(roots, mappings, descriptor, resource.fingerprints()) {
                Ok(found) => found,
                Err(detail) => return error_resolution(resource.id(), detail),
            };
        let state = match found.len() {
            0 if !diagnostics.is_empty() => ResourceResolutionState::Error,
            0 => ResourceResolutionState::Offline,
            1 => ResourceResolutionState::ResolvedProbable,
            _ => ResourceResolutionState::Ambiguous,
        };
        if state == ResourceResolutionState::Ambiguous {
            diagnostics.push(ResolutionEvidence::new(
                EvidenceKind::ConflictingCandidate,
                Some(format!("{} sequence directories match", found.len())),
            ));
        }
        let missing_frames = if found.len() == 1 {
            found[0].missing_frames.clone()
        } else {
            Vec::new()
        };
        ResourceResolution::new(
            resource.id(),
            state,
            found.into_iter().map(|found| found.candidate).collect(),
            diagnostics,
        )?
        .with_missing_frames(missing_frames)
    }

    fn discover_sequences(
        &self,
        roots: &[MediaRoot],
        mappings: &[MediaRootMapping],
        descriptor: &ImageSequenceDescriptor,
        fingerprints: &[ResourceFingerprint],
    ) -> std::result::Result<(Vec<SequenceCandidate>, Vec<ResolutionEvidence>), String> {
        let search = searchable_roots(roots, mappings)?;
        let mut found = BTreeMap::new();
        let mut entries_seen = 0_usize;
        for (root_name, root_path) in search.directories {
            for entry in WalkDir::new(&root_path)
                .follow_links(false)
                .max_depth(self.options.max_depth)
                .sort_by_file_name()
            {
                entries_seen = entries_seen.saturating_add(1);
                if entries_seen > self.options.max_entries {
                    return Err(format!(
                        "resolver entry limit {} exceeded",
                        self.options.max_entries
                    ));
                }
                let entry =
                    entry.map_err(|error| format!("scan {}: {error}", root_path.display()))?;
                if !entry.file_type().is_dir() {
                    continue;
                }
                let Some(candidate) =
                    verify_sequence_directory(entry.path(), &root_name, descriptor, fingerprints)?
                else {
                    continue;
                };
                found
                    .entry(candidate.candidate.uri().to_owned())
                    .or_insert(candidate);
            }
        }
        Ok((found.into_values().collect(), search.diagnostics))
    }

    fn discover(
        &self,
        roots: &[MediaRoot],
        mappings: &[MediaRootMapping],
        facts: Option<FileFacts>,
        has_fingerprint: bool,
        original_name: Option<&OsStr>,
    ) -> std::result::Result<Discovery, String> {
        let mut discovered = BTreeMap::new();
        let mut diagnostics = Vec::new();
        let mut entries_seen = 0_usize;
        let mut mappings_by_name = BTreeMap::new();
        for mapping in mappings {
            if mappings_by_name
                .insert(mapping.name(), mapping.directory())
                .is_some()
            {
                return Err(format!(
                    "media root {} has more than one machine mapping",
                    mapping.name()
                ));
            }
        }
        let mut roots: Vec<_> = roots.iter().filter(|root| root.is_enabled()).collect();
        roots.sort_by_key(|root| (root.priority(), root.id()));
        for root in roots {
            let legacy_path = root
                .legacy_uri()
                .map(file_uri_to_path)
                .transpose()
                .map_err(|error| {
                    format!("legacy media root {} is invalid: {error}", root.name())
                })?;
            let Some(root_path) = mappings_by_name
                .get(root.name())
                .copied()
                .or(legacy_path.as_deref())
            else {
                diagnostics.push(ResolutionEvidence::new(
                    EvidenceKind::MediaRootUnmapped,
                    Some(root.name().to_owned()),
                ));
                continue;
            };
            if !root_path.is_dir() {
                diagnostics.push(ResolutionEvidence::new(
                    EvidenceKind::MediaRootUnavailable,
                    Some(format!("{}: {}", root.name(), root_path.display())),
                ));
                continue;
            }
            for entry in WalkDir::new(root_path)
                .follow_links(false)
                .max_depth(self.options.max_depth)
                .sort_by_file_name()
            {
                entries_seen = entries_seen.saturating_add(1);
                if entries_seen > self.options.max_entries {
                    return Err(format!(
                        "resolver entry limit {} exceeded",
                        self.options.max_entries
                    ));
                }
                let entry = match entry {
                    Ok(entry) => entry,
                    Err(error) => {
                        diagnostics.push(ResolutionEvidence::new(
                            EvidenceKind::MediaRootUnavailable,
                            Some(format!("{}: {error}", root.name())),
                        ));
                        continue;
                    }
                };
                if !entry.file_type().is_file() {
                    continue;
                }
                let metadata = entry
                    .metadata()
                    .map_err(|error| format!("inspect {}: {error}", entry.path().display()))?;
                if facts.is_some_and(|facts| metadata.len() != facts.size_bytes()) {
                    continue;
                }
                let filename_matches = original_name.is_some_and(|name| entry.file_name() == name);
                if (!has_fingerprint || facts.is_none()) && !filename_matches {
                    continue;
                }
                let uri = canonical_file_uri(entry.path()).map_err(|error| error.to_string())?;
                let mut evidence = vec![ResolutionEvidence::new(
                    EvidenceKind::MediaRootRelation,
                    Some(root.name().to_owned()),
                )];
                if facts.is_some() {
                    evidence.push(ResolutionEvidence::new(EvidenceKind::FileSizeMatch, None));
                }
                if filename_matches {
                    evidence.push(ResolutionEvidence::new(EvidenceKind::FileNameMatch, None));
                }
                discovered.entry(uri).or_insert(evidence);
            }
        }
        Ok(Discovery {
            candidates: discovered,
            diagnostics,
        })
    }
}

fn searchable_roots(
    roots: &[MediaRoot],
    mappings: &[MediaRootMapping],
) -> std::result::Result<SearchableRoots, String> {
    let mut by_name = BTreeMap::new();
    for mapping in mappings {
        if by_name
            .insert(mapping.name(), mapping.directory())
            .is_some()
        {
            return Err(format!(
                "media root {} has more than one machine mapping",
                mapping.name()
            ));
        }
    }
    let mut ordered = roots
        .iter()
        .filter(|root| root.is_enabled())
        .collect::<Vec<_>>();
    ordered.sort_by_key(|root| (root.priority(), root.id()));
    let mut searchable = Vec::new();
    let mut diagnostics = Vec::new();
    for root in ordered {
        let legacy = root
            .legacy_uri()
            .map(file_uri_to_path)
            .transpose()
            .map_err(|error| format!("legacy media root {} is invalid: {error}", root.name()))?;
        let Some(path) = by_name.get(root.name()).copied().or(legacy.as_deref()) else {
            diagnostics.push(ResolutionEvidence::new(
                EvidenceKind::MediaRootUnmapped,
                Some(root.name().to_owned()),
            ));
            continue;
        };
        if path.is_dir() {
            searchable.push((root.name().to_owned(), path.to_path_buf()));
        } else {
            diagnostics.push(ResolutionEvidence::new(
                EvidenceKind::MediaRootUnavailable,
                Some(format!("{}: {}", root.name(), path.display())),
            ));
        }
    }
    Ok(SearchableRoots {
        directories: searchable,
        diagnostics,
    })
}

fn verify_sequence_directory(
    directory: &Path,
    root_name: &str,
    descriptor: &ImageSequenceDescriptor,
    fingerprints: &[ResourceFingerprint],
) -> std::result::Result<Option<SequenceCandidate>, String> {
    let missing_frames = sequence_missing_frames(directory, descriptor)?;
    if !missing_frames.is_empty() {
        return Ok(None);
    }
    let expected = fingerprints.iter().find(|fingerprint| {
        fingerprint.algorithm() == SEQUENCE_FINGERPRINT_ALGORITHM
            && fingerprint.version() == crate::SEQUENCE_FINGERPRINT_VERSION
    });
    let mut evidence = vec![
        ResolutionEvidence::new(EvidenceKind::MediaRootRelation, Some(root_name.to_owned())),
        ResolutionEvidence::new(
            EvidenceKind::FileNameMatch,
            Some(format!(
                "{}%0{}d{}",
                descriptor.pattern().prefix(),
                descriptor.pattern().padding(),
                descriptor.pattern().suffix()
            )),
        ),
    ];
    let confidence = if let Some(expected) = expected {
        let report =
            fingerprint_image_sequence(directory, descriptor).map_err(|error| error.to_string())?;
        if report.fingerprint() != expected {
            return Ok(None);
        }
        evidence.push(ResolutionEvidence::new(
            EvidenceKind::PartialFingerprintMatch,
            Some(format!(
                "{} version {}",
                expected.algorithm(),
                expected.version()
            )),
        ));
        Confidence::from_basis_points(9_500).map_err(|error| error.to_string())?
    } else {
        Confidence::from_basis_points(7_000).map_err(|error| error.to_string())?
    };
    let uri = canonical_file_uri(directory).map_err(|error| error.to_string())?;
    let candidate =
        ResolutionCandidate::new(uri, confidence, evidence).map_err(|error| error.to_string())?;
    Ok(Some(SequenceCandidate {
        candidate,
        missing_frames,
    }))
}

fn online_known_candidate(known_locators: &[Locator]) -> Result<Option<ResolutionCandidate>> {
    let mut online = Vec::new();
    for locator in known_locators {
        let Ok(path) = file_uri_to_path(locator.uri()) else {
            continue;
        };
        if path.is_file() {
            online.push(ResolutionCandidate::new(
                locator.uri(),
                Confidence::CERTAIN,
                vec![ResolutionEvidence::new(
                    EvidenceKind::KnownLocatorAvailable,
                    None,
                )],
            )?);
        }
    }
    online.sort_by(|left, right| left.uri().cmp(right.uri()));
    Ok(online.into_iter().next())
}

fn online_sequence_candidate(
    known_locators: &[Locator],
    descriptor: &ImageSequenceDescriptor,
) -> std::result::Result<Option<(ResolutionCandidate, Vec<i64>)>, String> {
    let mut online = known_locators
        .iter()
        .filter_map(|locator| {
            let path = file_uri_to_path(locator.uri()).ok()?;
            path.is_dir().then_some((locator.uri(), path))
        })
        .collect::<Vec<_>>();
    online.sort_by_key(|(uri, _)| *uri);
    let Some((uri, path)) = online.into_iter().next() else {
        return Ok(None);
    };

    let missing_frames = sequence_missing_frames(&path, descriptor)?;

    let candidate = ResolutionCandidate::new(
        uri,
        Confidence::CERTAIN,
        vec![ResolutionEvidence::new(
            EvidenceKind::KnownLocatorAvailable,
            None,
        )],
    )
    .map_err(|error| error.to_string())?;
    Ok(Some((candidate, missing_frames)))
}

fn sequence_missing_frames(
    path: &Path,
    descriptor: &ImageSequenceDescriptor,
) -> std::result::Result<Vec<i64>, String> {
    let names = fs::read_dir(path)
        .map_err(|error| format!("list image-sequence directory {}: {error}", path.display()))?
        .map(|entry| {
            entry.map(|entry| entry.file_name()).map_err(|error| {
                format!("read image-sequence directory {}: {error}", path.display())
            })
        })
        .collect::<std::result::Result<BTreeSet<_>, _>>()?;
    let mut missing_frames = Vec::new();
    let frames = descriptor.frames();
    let mut frame = frames.start();
    loop {
        if !descriptor.is_known_missing(frame)
            && !names.contains(OsStr::new(&descriptor.pattern().filename(frame)))
        {
            missing_frames.push(frame);
            if missing_frames.len() > MAX_SEQUENCE_EXCEPTIONS {
                return Err(format!(
                    "image sequence has more than {MAX_SEQUENCE_EXCEPTIONS} missing frames"
                ));
            }
        }
        if frame == frames.end() {
            break;
        }
        frame += i64::from(frames.step());
    }

    Ok(missing_frames)
}

fn verify_candidate(
    path: &Path,
    uri: &str,
    mut evidence: Vec<ResolutionEvidence>,
    expected: &[ResourceFingerprint],
) -> std::result::Result<Option<ResolutionCandidate>, String> {
    if !expected.is_empty() {
        let report = fingerprint_file(path).map_err(|error| error.to_string())?;
        let Some(expected) = expected.iter().find(|expected| {
            expected.algorithm() == report.fingerprint().algorithm()
                && expected.version() == report.fingerprint().version()
        }) else {
            return Ok(None);
        };
        if report.fingerprint() != expected {
            return Ok(None);
        }
        let full = expected.algorithm() == FULL_FINGERPRINT_ALGORITHM;
        evidence.push(ResolutionEvidence::new(
            if full {
                EvidenceKind::FullHashMatch
            } else {
                EvidenceKind::PartialFingerprintMatch
            },
            Some(format!(
                "{} version {}",
                expected.algorithm(),
                expected.version()
            )),
        ));
        if full {
            evidence.push(ResolutionEvidence::new(
                EvidenceKind::ExactFingerprintMatch,
                None,
            ));
        }
        let confidence = if full {
            Confidence::CERTAIN
        } else {
            Confidence::from_basis_points(9_500).map_err(|error| error.to_string())?
        };
        return ResolutionCandidate::new(uri, confidence, evidence)
            .map(Some)
            .map_err(|error| error.to_string());
    }

    let filename_match = evidence
        .iter()
        .any(|item| item.kind() == EvidenceKind::FileNameMatch);
    if !filename_match {
        return Ok(None);
    }
    let confidence = Confidence::from_basis_points(
        if evidence
            .iter()
            .any(|item| item.kind() == EvidenceKind::FileSizeMatch)
        {
            7_000
        } else {
            5_000
        },
    )
    .map_err(|error| error.to_string())?;
    ResolutionCandidate::new(uri, confidence, evidence)
        .map(Some)
        .map_err(|error| error.to_string())
}

fn error_resolution(
    resource_id: postproject_core::ResourceId,
    detail: String,
) -> Result<ResourceResolution> {
    ResourceResolution::new(
        resource_id,
        ResourceResolutionState::Error,
        Vec::new(),
        vec![ResolutionEvidence::new(
            EvidenceKind::DiscoveryError,
            Some(detail),
        )],
    )
}

fn file_uri_to_path(uri: &str) -> Result<PathBuf> {
    let url = Url::parse(uri).map_err(|error| {
        Error::new(
            ErrorKind::InvalidArgument,
            format!("invalid file URI: {error}"),
        )
    })?;
    url.to_file_path().map_err(|()| {
        Error::new(
            ErrorKind::Unsupported,
            format!("URI is not a local file path: {uri}"),
        )
    })
}

#[cfg(test)]
mod tests {
    use std::fs;

    use postproject_core::{
        ContentStructure, FrameRange, ImageSequenceDescriptor, ImageSequencePattern, LocatorId,
        MediaRoot, MediaRootId, RationalRate, RepresentationAvailability, RepresentationId,
        RepresentationResolution, ResourceId, ResourceResolutionState,
    };

    use super::*;
    use crate::{
        ImageSequenceSource, prepare_image_sequence_representation, prepare_media_root,
        prepare_original_media,
    };

    #[test]
    fn known_online_locator_wins_without_root_scan() {
        let directory = tempfile::tempdir().expect("create directory");
        let path = directory.path().join("clip.mov");
        fs::write(&path, b"media").expect("write media");
        let prepared = prepare_original_media(&path, None, None).expect("prepare import");

        let resolution = MediaResolver::default()
            .resolve_resource(
                &prepared.resources()[0],
                prepared.representation().content_structure(),
                prepared.locators(),
                &[],
                &[],
            )
            .expect("resolve known locator");

        assert_eq!(
            resolution.state(),
            ResourceResolutionState::OnlineAtKnownLocator
        );
        assert_eq!(
            resolution.candidates()[0].uri(),
            prepared.locators()[0].uri()
        );
    }

    #[test]
    fn sequence_directory_is_online_and_recorded_gaps_are_partial() {
        let directory = tempfile::tempdir().expect("create directory");
        let sequence_directory = directory.path().join("plate");
        fs::create_dir(&sequence_directory).expect("create sequence directory");
        fs::write(sequence_directory.join("plate.0001.exr"), b"frame 1")
            .expect("write first frame");
        fs::write(sequence_directory.join("plate.0003.exr"), b"frame 3")
            .expect("write third frame");
        let resource_id = ResourceId::new();
        let descriptor = ImageSequenceDescriptor::new(
            resource_id,
            ImageSequencePattern::new("plate.", ".exr", 4).expect("valid pattern"),
            FrameRange::new(1, 3, 1).expect("valid frame range"),
            RationalRate::new(24, 1).expect("valid rate"),
            vec![2],
        )
        .expect("valid sequence");
        let structure = ContentStructure::image_sequence(descriptor);
        let resource = Resource::new(resource_id, Vec::new(), None);
        let locator = Locator::new(
            LocatorId::new(),
            resource_id,
            canonical_file_uri(&sequence_directory).expect("sequence URI"),
            None,
            postproject_core::LocatorAvailability::Online,
        )
        .expect("valid locator");

        let resolved = MediaResolver::default()
            .resolve_resource(&resource, &structure, &[locator], &[], &[])
            .expect("resolve known sequence directory");
        assert_eq!(
            resolved.state(),
            ResourceResolutionState::OnlineAtKnownLocator
        );
        let aggregate = RepresentationResolution::aggregate(
            RepresentationId::new(),
            &structure,
            vec![resolved],
        )
        .expect("aggregate sequence");
        assert_eq!(
            aggregate.availability(),
            RepresentationAvailability::Partial
        );
        assert_eq!(aggregate.issues()[0].frames(), &[2]);
    }

    #[test]
    fn sequence_directory_inventory_reports_unrecorded_gaps() {
        let directory = tempfile::tempdir().expect("create directory");
        fs::write(directory.path().join("plate.1001.exr"), b"frame 1001")
            .expect("write first frame");
        fs::write(directory.path().join("plate.1003.exr"), b"frame 1003")
            .expect("write third frame");
        let resource_id = ResourceId::new();
        let descriptor = ImageSequenceDescriptor::new(
            resource_id,
            ImageSequencePattern::new("plate.", ".exr", 4).expect("valid pattern"),
            FrameRange::new(1001, 1004, 1).expect("valid frame range"),
            RationalRate::new(24, 1).expect("valid rate"),
            Vec::new(),
        )
        .expect("valid sequence");
        let structure = ContentStructure::image_sequence(descriptor);
        let resource = Resource::new(resource_id, Vec::new(), None);
        let locator = Locator::new(
            LocatorId::new(),
            resource_id,
            canonical_file_uri(directory.path()).expect("sequence URI"),
            None,
            postproject_core::LocatorAvailability::Online,
        )
        .expect("valid locator");

        let resolved = MediaResolver::default()
            .resolve_resource(&resource, &structure, &[locator], &[], &[])
            .expect("resolve known sequence directory");
        assert_eq!(resolved.missing_frames(), &[1002, 1004]);
        let aggregate = RepresentationResolution::aggregate(
            RepresentationId::new(),
            &structure,
            vec![resolved],
        )
        .expect("aggregate sequence");

        assert_eq!(
            aggregate.availability(),
            RepresentationAvailability::Partial
        );
        assert_eq!(aggregate.issues()[0].frames(), &[1002, 1004]);
    }

    #[test]
    fn finds_a_relocated_sequence_as_one_resource() {
        let temporary = tempfile::tempdir().expect("create directory");
        let original = temporary.path().join("original");
        let mapped_root = temporary.path().join("mapped");
        fs::create_dir(&original).expect("create original sequence");
        fs::create_dir(&mapped_root).expect("create mapped root");
        for frame in 1001..=1003 {
            fs::write(
                original.join(format!("plate.{frame}.exr")),
                frame.to_string(),
            )
            .expect("write frame");
        }
        let prepared = prepare_image_sequence_representation(
            postproject_core::AssetId::new(),
            postproject_core::RepresentationKind::Original,
            &ImageSequenceSource::new(
                &original,
                ImageSequencePattern::new("plate.", ".exr", 4).expect("pattern"),
                FrameRange::new(1001, 1003, 1).expect("range"),
                RationalRate::new(24, 1).expect("rate"),
                Vec::new(),
            ),
        )
        .expect("prepare sequence");
        let relocated = mapped_root.join("cards/day-01/plate");
        fs::create_dir_all(relocated.parent().expect("parent")).expect("create parent");
        fs::rename(&original, &relocated).expect("relocate sequence");
        let root = MediaRoot::new(MediaRootId::new(), "rushes", None, None, 0, true)
            .expect("portable root");
        let mapping = MediaRootMapping::new("rushes", &mapped_root).expect("root mapping");

        let resolved = MediaResolver::default()
            .resolve_resource(
                &prepared.resources()[0],
                prepared.representation().content_structure(),
                prepared.locators(),
                &[root],
                &[mapping],
            )
            .expect("resolve moved sequence");
        assert_eq!(resolved.state(), ResourceResolutionState::ResolvedProbable);
        assert_eq!(resolved.candidates().len(), 1);
        assert_eq!(
            resolved.candidates()[0].uri(),
            canonical_file_uri(&relocated).expect("relocated URI")
        );
    }

    #[test]
    fn moved_small_file_resolves_by_full_hash() {
        let directory = tempfile::tempdir().expect("create directory");
        let old_directory = directory.path().join("old");
        let new_directory = directory.path().join("new");
        fs::create_dir(&old_directory).expect("create old directory");
        fs::create_dir(&new_directory).expect("create new directory");
        let old_path = old_directory.join("clip.mov");
        let new_path = new_directory.join("renamed.mov");
        fs::write(&old_path, b"media").expect("write media");
        let prepared = prepare_original_media(&old_path, None, None).expect("prepare import");
        fs::rename(&old_path, &new_path).expect("move media");
        let root = prepare_media_root(&new_directory, None, 0).expect("prepare root");

        let resolution = MediaResolver::default()
            .resolve_resource(
                &prepared.resources()[0],
                prepared.representation().content_structure(),
                prepared.locators(),
                &[root],
                &[],
            )
            .expect("resolve moved media");

        assert_eq!(resolution.state(), ResourceResolutionState::ResolvedExact);
        assert_eq!(resolution.candidates().len(), 1);
        assert!(
            resolution.candidates()[0]
                .evidence()
                .iter()
                .any(|evidence| evidence.kind() == EvidenceKind::FullHashMatch)
        );
    }

    #[test]
    fn machine_mapping_resolves_a_portable_root() {
        let directory = tempfile::tempdir().expect("create directory");
        let old_directory = directory.path().join("workstation");
        let laptop_directory = directory.path().join("laptop");
        fs::create_dir(&old_directory).expect("create workstation directory");
        fs::create_dir(&laptop_directory).expect("create laptop directory");
        let old_path = old_directory.join("clip.mov");
        fs::write(&old_path, b"portable media").expect("write media");
        let prepared = prepare_original_media(&old_path, None, None).expect("prepare import");
        fs::rename(&old_path, laptop_directory.join("clip.mov")).expect("move media");
        let root = MediaRoot::new(
            MediaRootId::new(),
            "camera-originals",
            Some("Camera originals".to_owned()),
            None,
            0,
            true,
        )
        .expect("create portable root");
        let mapping =
            MediaRootMapping::new("camera-originals", &laptop_directory).expect("map root");

        let resolution = MediaResolver::default()
            .resolve_resource(
                &prepared.resources()[0],
                prepared.representation().content_structure(),
                prepared.locators(),
                &[root],
                &[mapping],
            )
            .expect("resolve through mapping");

        assert_eq!(resolution.state(), ResourceResolutionState::ResolvedExact);
        assert_eq!(resolution.candidates().len(), 1);
    }

    #[test]
    fn unusable_roots_are_reported_without_hiding_reachable_results() {
        let directory = tempfile::tempdir().expect("create directory");
        let old_path = directory.path().join("old.mov");
        let reachable = directory.path().join("reachable");
        fs::create_dir(&reachable).expect("create reachable root");
        fs::write(&old_path, b"media").expect("write original");
        let prepared = prepare_original_media(&old_path, None, None).expect("prepare import");
        fs::rename(&old_path, reachable.join("moved.mov")).expect("move media");
        let unmapped = MediaRoot::new(MediaRootId::new(), "archive", None, None, 0, true)
            .expect("create unmapped root");
        let usable = MediaRoot::new(MediaRootId::new(), "working", None, None, 1, true)
            .expect("create mapped root");
        let mapping = MediaRootMapping::new("working", &reachable).expect("map reachable root");

        let resolution = MediaResolver::default()
            .resolve_resource(
                &prepared.resources()[0],
                prepared.representation().content_structure(),
                prepared.locators(),
                &[unmapped, usable],
                &[mapping],
            )
            .expect("resolve reachable root");

        assert_eq!(resolution.state(), ResourceResolutionState::ResolvedExact);
        assert!(resolution.evidence().iter().any(|evidence| {
            evidence.kind() == EvidenceKind::MediaRootUnmapped
                && evidence.detail() == Some("archive")
        }));
    }

    #[test]
    fn unmapped_root_is_not_reported_as_missing_media() {
        let directory = tempfile::tempdir().expect("create directory");
        let path = directory.path().join("clip.mov");
        fs::write(&path, b"media").expect("write media");
        let prepared = prepare_original_media(&path, None, None).expect("prepare import");
        fs::remove_file(path).expect("remove known media");
        let root = MediaRoot::new(MediaRootId::new(), "offline-vault", None, None, 0, true)
            .expect("create root");

        let resolution = MediaResolver::default()
            .resolve_resource(
                &prepared.resources()[0],
                prepared.representation().content_structure(),
                prepared.locators(),
                &[root],
                &[],
            )
            .expect("report unmapped root");

        assert_eq!(resolution.state(), ResourceResolutionState::Error);
        assert_eq!(
            resolution.evidence()[0].kind(),
            EvidenceKind::MediaRootUnmapped
        );
    }

    #[test]
    fn identical_copies_are_ambiguous_and_deterministically_ordered() {
        let directory = tempfile::tempdir().expect("create directory");
        let old_path = directory.path().join("old.mov");
        fs::write(&old_path, b"identical media").expect("write original");
        let prepared = prepare_original_media(&old_path, None, None).expect("prepare import");
        fs::remove_file(&old_path).expect("remove old locator target");
        fs::write(directory.path().join("b.mov"), b"identical media").expect("write b");
        fs::write(directory.path().join("a.mov"), b"identical media").expect("write a");
        let root = prepare_media_root(directory.path(), None, 0).expect("prepare root");

        let resolution = MediaResolver::default()
            .resolve_resource(
                &prepared.resources()[0],
                prepared.representation().content_structure(),
                prepared.locators(),
                &[root],
                &[],
            )
            .expect("resolve ambiguous media");

        assert_eq!(resolution.state(), ResourceResolutionState::Ambiguous);
        assert_eq!(resolution.candidates().len(), 2);
        assert!(resolution.candidates()[0].uri() < resolution.candidates()[1].uri());
    }

    #[test]
    fn scan_limit_is_an_explainable_error_result() {
        let directory = tempfile::tempdir().expect("create directory");
        let old_path = directory.path().join("old.mov");
        fs::write(&old_path, b"media").expect("write original");
        let prepared = prepare_original_media(&old_path, None, None).expect("prepare import");
        fs::remove_file(&old_path).expect("remove old locator target");
        fs::write(directory.path().join("candidate.mov"), b"media").expect("write candidate");
        let root = prepare_media_root(directory.path(), None, 0).expect("prepare root");
        let resolver = MediaResolver::new(ResolverOptions {
            max_depth: 4,
            max_entries: 1,
        })
        .expect("valid limits");

        let resolution = resolver
            .resolve_resource(
                &prepared.resources()[0],
                prepared.representation().content_structure(),
                prepared.locators(),
                &[root],
                &[],
            )
            .expect("create error result");

        assert_eq!(resolution.state(), ResourceResolutionState::Error);
        assert_eq!(
            resolution.evidence()[0].kind(),
            EvidenceKind::DiscoveryError
        );
    }
}
