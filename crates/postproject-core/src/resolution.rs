//! Explainable and deterministic media-resolution results.

use std::cmp::Reverse;

use crate::{Error, ErrorKind, RepresentationId, Result};

/// A deterministic confidence value in basis points from 0 through 10,000.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Confidence(u16);

impl Confidence {
    /// Certain confidence used only for verified identity evidence.
    pub const CERTAIN: Self = Self(10_000);

    /// Creates a confidence value, rejecting values above 100%.
    ///
    /// # Errors
    ///
    /// Returns [`ErrorKind::InvalidArgument`] when `value` exceeds 10,000.
    pub fn from_basis_points(value: u16) -> Result<Self> {
        if value > 10_000 {
            return Err(Error::new(
                ErrorKind::InvalidArgument,
                "confidence must not exceed 10,000 basis points",
            ));
        }
        Ok(Self(value))
    }

    /// Returns the confidence in basis points.
    #[must_use]
    pub const fn basis_points(self) -> u16 {
        self.0
    }
}

/// A machine-inspectable reason supporting or opposing a resolution candidate.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[non_exhaustive]
pub enum EvidenceKind {
    /// A persisted location currently exists.
    KnownLocationExists,
    /// The complete stored fingerprint matches.
    ExactFingerprintMatch,
    /// A cryptographic full-file digest matches.
    FullHashMatch,
    /// A sampled or otherwise partial fingerprint matches.
    PartialFingerprintMatch,
    /// The byte size matches.
    FileSizeMatch,
    /// The final path component matches.
    FileNameMatch,
    /// The path relative to a media root is similar.
    RelativePathSimilarity,
    /// The candidate is contained in a configured media root.
    MediaRootRelation,
    /// Another candidate has equivalent credible evidence.
    ConflictingCandidate,
}

/// Structured evidence with optional human-readable context.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResolutionEvidence {
    kind: EvidenceKind,
    detail: Option<String>,
}

impl ResolutionEvidence {
    /// Creates a structured evidence item.
    #[must_use]
    pub fn new(kind: EvidenceKind, detail: Option<String>) -> Self {
        Self { kind, detail }
    }

    /// Returns the machine-readable evidence kind.
    #[must_use]
    pub const fn kind(&self) -> EvidenceKind {
        self.kind
    }

    /// Returns optional diagnostic context.
    #[must_use]
    pub fn detail(&self) -> Option<&str> {
        self.detail.as_deref()
    }
}

/// A possible physical location considered by the resolver.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResolutionCandidate {
    uri: String,
    confidence: Confidence,
    evidence: Vec<ResolutionEvidence>,
}

impl ResolutionCandidate {
    /// Creates a candidate with a non-empty URI and at least one evidence item.
    ///
    /// # Errors
    ///
    /// Returns [`ErrorKind::InvalidArgument`] when `uri` or `evidence` is empty.
    pub fn new(
        uri: impl Into<String>,
        confidence: Confidence,
        evidence: Vec<ResolutionEvidence>,
    ) -> Result<Self> {
        let uri = uri.into();
        if uri.is_empty() {
            return Err(Error::new(
                ErrorKind::InvalidArgument,
                "resolution candidate URI must not be empty",
            ));
        }
        if evidence.is_empty() {
            return Err(Error::new(
                ErrorKind::InvalidArgument,
                "resolution candidate must contain evidence",
            ));
        }
        Ok(Self {
            uri,
            confidence,
            evidence,
        })
    }

    /// Returns the candidate URI.
    #[must_use]
    pub fn uri(&self) -> &str {
        &self.uri
    }

    /// Returns the deterministic confidence value.
    #[must_use]
    pub const fn confidence(&self) -> Confidence {
        self.confidence
    }

    /// Returns the inspectable supporting evidence.
    #[must_use]
    pub fn evidence(&self) -> &[ResolutionEvidence] {
        &self.evidence
    }
}

/// The outcome category of a media-resolution attempt.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum ResolutionState {
    /// A stored location remains online, so no scan was required.
    OnlineAtKnownLocation,
    /// One candidate has exact identity evidence.
    ResolvedExact,
    /// One candidate is credible but lacks exact verification.
    ResolvedProbable,
    /// No credible candidate was found.
    Missing,
    /// Multiple candidates are equally credible and require confirmation.
    Ambiguous,
    /// Candidate discovery or verification failed.
    Error,
}

/// An explainable resolution result with deterministically ordered candidates.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Resolution {
    representation_id: RepresentationId,
    state: ResolutionState,
    candidates: Vec<ResolutionCandidate>,
    evidence: Vec<ResolutionEvidence>,
}

impl Resolution {
    /// Creates a result and sorts candidates by descending confidence then URI.
    ///
    /// Ambiguous results require at least two candidates. Successful single-choice
    /// states require exactly one. Missing and error results select no candidate.
    ///
    /// # Errors
    ///
    /// Returns [`ErrorKind::InvalidArgument`] when the candidate count is
    /// inconsistent with `state`.
    pub fn new(
        representation_id: RepresentationId,
        state: ResolutionState,
        mut candidates: Vec<ResolutionCandidate>,
        evidence: Vec<ResolutionEvidence>,
    ) -> Result<Self> {
        candidates.sort_by(|left, right| {
            (Reverse(left.confidence), left.uri.as_str())
                .cmp(&(Reverse(right.confidence), right.uri.as_str()))
        });

        let valid_count = match state {
            ResolutionState::OnlineAtKnownLocation
            | ResolutionState::ResolvedExact
            | ResolutionState::ResolvedProbable => candidates.len() == 1,
            ResolutionState::Missing | ResolutionState::Error => candidates.is_empty(),
            ResolutionState::Ambiguous => candidates.len() >= 2,
        };
        if !valid_count {
            return Err(Error::new(
                ErrorKind::InvalidArgument,
                "candidate count is inconsistent with resolution state",
            ));
        }

        Ok(Self {
            representation_id,
            state,
            candidates,
            evidence,
        })
    }

    /// Returns the representation that was resolved.
    #[must_use]
    pub const fn representation_id(&self) -> RepresentationId {
        self.representation_id
    }

    /// Returns the outcome category.
    #[must_use]
    pub const fn state(&self) -> ResolutionState {
        self.state
    }

    /// Returns candidates in deterministic best-first order.
    #[must_use]
    pub fn candidates(&self) -> &[ResolutionCandidate] {
        &self.candidates
    }

    /// Returns result-wide evidence and diagnostics.
    #[must_use]
    pub fn evidence(&self) -> &[ResolutionEvidence] {
        &self.evidence
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;

    fn candidate(uri: &str, confidence: u16) -> ResolutionCandidate {
        ResolutionCandidate::new(
            uri,
            Confidence::from_basis_points(confidence).expect("test confidence is valid"),
            vec![ResolutionEvidence::new(EvidenceKind::FileSizeMatch, None)],
        )
        .expect("test candidate is valid")
    }

    #[test]
    fn candidates_are_sorted_deterministically() {
        let resolution = Resolution::new(
            RepresentationId::new(),
            ResolutionState::Ambiguous,
            vec![
                candidate("file:///z", 8_000),
                candidate("file:///b", 9_000),
                candidate("file:///a", 9_000),
            ],
            Vec::new(),
        )
        .expect("ambiguous result has enough candidates");

        let uris: Vec<_> = resolution
            .candidates()
            .iter()
            .map(ResolutionCandidate::uri)
            .collect();
        assert_eq!(uris, ["file:///a", "file:///b", "file:///z"]);
    }

    #[test]
    fn ambiguity_cannot_silently_select_one_candidate() {
        let error = Resolution::new(
            RepresentationId::new(),
            ResolutionState::Ambiguous,
            vec![candidate("file:///only", 10_000)],
            Vec::new(),
        )
        .expect_err("one candidate cannot be ambiguous");
        assert_eq!(error.kind(), ErrorKind::InvalidArgument);
    }

    proptest! {
        #[test]
        fn ordering_is_invariant_under_input_reversal(
            left_score in 0_u16..=10_000,
            right_score in 0_u16..=10_000,
        ) {
            let id = RepresentationId::from_bytes([4; 16]);
            let forward = Resolution::new(
                id,
                ResolutionState::Ambiguous,
                vec![candidate("file:///a", left_score), candidate("file:///b", right_score)],
                Vec::new(),
            ).expect("valid resolution");
            let reverse = Resolution::new(
                id,
                ResolutionState::Ambiguous,
                vec![candidate("file:///b", right_score), candidate("file:///a", left_score)],
                Vec::new(),
            ).expect("valid resolution");

            prop_assert_eq!(forward, reverse);
        }
    }
}
