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

use crate::{FULL_FINGERPRINT_ALGORITHM, canonical_file_uri, fingerprint_file};

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
                        return ResourceResolution::new(
                            resource.id(),
                            ResourceResolutionState::Offline,
                            Vec::new(),
                            Vec::new(),
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
            resource.file_facts(),
            !resource.fingerprints().is_empty(),
            original_name.as_deref(),
        ) {
            Ok(discovered) => discovered,
            Err(detail) => return error_resolution(resource.id(), detail),
        };

        let mut candidates = Vec::new();
        for (uri, cheap_evidence) in discovered {
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
            0 => ResourceResolutionState::Offline,
            1 if candidates[0].confidence() == Confidence::CERTAIN => {
                ResourceResolutionState::ResolvedExact
            }
            1 => ResourceResolutionState::ResolvedProbable,
            _ => ResourceResolutionState::Ambiguous,
        };
        let evidence = if state == ResourceResolutionState::Ambiguous {
            vec![ResolutionEvidence::new(
                EvidenceKind::ConflictingCandidate,
                Some(format!(
                    "{} candidates have matching identity evidence",
                    candidates.len()
                )),
            )]
        } else {
            Vec::new()
        };
        ResourceResolution::new(resource.id(), state, candidates, evidence)
    }

    fn discover(
        &self,
        roots: &[MediaRoot],
        facts: Option<FileFacts>,
        has_fingerprint: bool,
        original_name: Option<&OsStr>,
    ) -> std::result::Result<BTreeMap<String, Vec<ResolutionEvidence>>, String> {
        let mut discovered = BTreeMap::new();
        let mut entries_seen = 0_usize;
        let mut roots: Vec<_> = roots.iter().filter(|root| root.is_enabled()).collect();
        roots.sort_by_key(|root| (root.priority(), root.id()));
        for root in roots {
            let Some(root_uri) = root.legacy_uri() else {
                return Err(format!("media root {} is unmapped", root.name()));
            };
            let root_path = file_uri_to_path(root_uri).map_err(|error| {
                format!(
                    "media root {} is not a local file URI: {error}",
                    root.name()
                )
            })?;
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
        Ok(discovered)
    }
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

    let names = fs::read_dir(&path)
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
        RationalRate, RepresentationAvailability, RepresentationId, RepresentationResolution,
        ResourceId, ResourceResolutionState,
    };

    use super::*;
    use crate::{prepare_media_root, prepare_original_media};

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
            .resolve_resource(&resource, &structure, &[locator], &[])
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
            .resolve_resource(&resource, &structure, &[locator], &[])
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
            )
            .expect("create error result");

        assert_eq!(resolution.state(), ResourceResolutionState::Error);
        assert_eq!(
            resolution.evidence()[0].kind(),
            EvidenceKind::DiscoveryError
        );
    }
}
