//! Derived knowledge state for activity-produced representations.

use crate::{
    ActivityId, ActivityKind, DependencyKind, DependencyTarget, RepresentationId, ResourceId,
};

/// One authored edge in a dependency path captured for an activity input.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArtifactDependencyPathSegment {
    source_representation_id: RepresentationId,
    dependency_position: u32,
    source_resource_id: Option<ResourceId>,
    kind: DependencyKind,
    target: DependencyTarget,
    resolved_representation_id: Option<RepresentationId>,
    authored_reference: String,
}

impl ArtifactDependencyPathSegment {
    /// Reconstructs one storage-captured path segment.
    #[doc(hidden)]
    #[must_use]
    pub fn new(
        source_representation_id: RepresentationId,
        dependency_position: u32,
        source_resource_id: Option<ResourceId>,
        kind: DependencyKind,
        target: DependencyTarget,
        resolved_representation_id: Option<RepresentationId>,
        authored_reference: String,
    ) -> Self {
        Self {
            source_representation_id,
            dependency_position,
            source_resource_id,
            kind,
            target,
            resolved_representation_id,
            authored_reference,
        }
    }

    /// Returns the representation containing this edge.
    #[must_use]
    pub const fn source_representation_id(&self) -> RepresentationId {
        self.source_representation_id
    }

    /// Returns the edge's position in its source dependency observation.
    #[must_use]
    pub const fn dependency_position(&self) -> u32 {
        self.dependency_position
    }

    /// Returns the optional resource that authored the reference.
    #[must_use]
    pub const fn source_resource_id(&self) -> Option<ResourceId> {
        self.source_resource_id
    }

    /// Returns the open-world dependency kind.
    #[must_use]
    pub const fn kind(&self) -> &DependencyKind {
        &self.kind
    }

    /// Returns the floating or pinned target.
    #[must_use]
    pub const fn target(&self) -> DependencyTarget {
        self.target
    }

    /// Returns the representation selected for a floating target.
    #[must_use]
    pub const fn resolved_representation_id(&self) -> Option<RepresentationId> {
        self.resolved_representation_id
    }

    /// Returns the exact authored reference text.
    #[must_use]
    pub fn authored_reference(&self) -> &str {
        &self.authored_reference
    }
}

/// Why captured dependency evidence cannot prove currentness.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum ArtifactDependencyIssue {
    /// A source dependency observation must be extracted again.
    NeedsExtraction,
    /// A floating asset target had no resolved representation.
    Unresolved,
    /// Capture reached the fixed dependency-depth bound.
    DepthTruncated,
    /// Capture reached the fixed dependency-representation bound.
    RepresentationsTruncated,
}

/// Knowledge state of a representation produced by an activity.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum ArtifactKnowledgeState {
    /// Every comparable input and output fingerprint still matches.
    Current,
    /// An input changed or an upstream artifact is not current.
    Stale,
    /// Stored evidence is insufficient to decide whether the artifact is current.
    Indeterminate,
    /// The output itself differs from the fingerprint captured when produced.
    Diverged,
}

/// Which side of an activity supplied fingerprint evidence.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum ArtifactEdgeKind {
    /// A representation consumed by the activity.
    Input,
    /// A representation produced by the activity.
    Output,
}

/// Which explicit traversal bound stopped evaluation.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum ArtifactTraversalLimitKind {
    /// The maximum upstream depth was reached.
    Depth,
    /// The maximum number of visited representations was reached.
    Representations,
}

/// One user-explainable reason for a non-current knowledge state.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum ArtifactKnowledgeReason {
    /// No activity records how the target representation was produced.
    ProducingActivityMissing {
        /// Representation being evaluated.
        representation_id: RepresentationId,
    },
    /// More than one activity claims to have produced the representation.
    ProducingActivityAmbiguous {
        /// Representation being evaluated.
        representation_id: RepresentationId,
        /// Number of producing activities found.
        activity_count: u32,
    },
    /// A legacy activity edge has no storage-captured snapshot.
    SnapshotAbsent {
        /// Activity owning the edge.
        activity_id: ActivityId,
        /// Representation referenced by the edge.
        representation_id: RepresentationId,
        /// Whether this was an input or output edge.
        edge: ArtifactEdgeKind,
    },
    /// A legacy activity input has no dependency-closure snapshot.
    DependencySnapshotAbsent {
        /// Activity owning the input.
        activity_id: ActivityId,
        /// Input representation whose closure was not captured.
        representation_id: RepresentationId,
    },
    /// Captured dependency knowledge was incomplete at activity creation.
    DependencyKnowledgeIncomplete {
        /// Activity owning the input.
        activity_id: ActivityId,
        /// Direct activity input.
        input_representation_id: RepresentationId,
        /// Representation at which capture became indeterminate.
        subject_representation_id: RepresentationId,
        /// Captured typed path to the subject or unresolved edge.
        path: Vec<ArtifactDependencyPathSegment>,
        /// Specific incomplete-knowledge condition.
        issue: ArtifactDependencyIssue,
    },
    /// A required authored path no longer matches its captured observation.
    DependencyPathChanged {
        /// Activity owning the input.
        activity_id: ActivityId,
        /// Direct activity input.
        input_representation_id: RepresentationId,
        /// Captured path that changed.
        path: Vec<ArtifactDependencyPathSegment>,
    },
    /// Current dependency content differs from the captured fingerprint.
    DependencyFingerprintChanged {
        /// Activity owning the input.
        activity_id: ActivityId,
        /// Direct activity input.
        input_representation_id: RepresentationId,
        /// Changed dependency representation.
        representation_id: RepresentationId,
        /// Captured typed path to the dependency.
        path: Vec<ArtifactDependencyPathSegment>,
        /// Fingerprint algorithm.
        algorithm: String,
        /// Fingerprint format version.
        version: u16,
        /// Captured fingerprint bytes.
        snapshot_value: Vec<u8>,
        /// Current fingerprint bytes.
        current_value: Vec<u8>,
    },
    /// A dependency lacks comparable captured or current fingerprint evidence.
    DependencyFingerprintEvidenceMissing {
        /// Activity owning the input.
        activity_id: ActivityId,
        /// Direct activity input.
        input_representation_id: RepresentationId,
        /// Dependency representation lacking evidence.
        representation_id: RepresentationId,
        /// Captured typed path to the dependency.
        path: Vec<ArtifactDependencyPathSegment>,
        /// Algorithm domain, when either side supplied one.
        algorithm: Option<String>,
        /// Algorithm version, when either side supplied one.
        version: Option<u16>,
        /// Captured value, when present.
        snapshot_value: Option<Vec<u8>>,
        /// Current value, when present.
        current_value: Option<Vec<u8>>,
    },
    /// The edge has no comparable fingerprint evidence.
    FingerprintEvidenceMissing {
        /// Activity owning the edge.
        activity_id: ActivityId,
        /// Representation referenced by the edge.
        representation_id: RepresentationId,
        /// Whether this was an input or output edge.
        edge: ArtifactEdgeKind,
        /// Algorithm domain, when one side of the comparison had it.
        algorithm: Option<String>,
        /// Algorithm version, when one side of the comparison had it.
        version: Option<u16>,
        /// Snapshot value, when present.
        snapshot_value: Option<Vec<u8>>,
        /// Current value, when present.
        current_value: Option<Vec<u8>>,
    },
    /// Current content differs from the edge snapshot in one fingerprint domain.
    FingerprintChanged {
        /// Activity owning the edge.
        activity_id: ActivityId,
        /// Representation referenced by the edge.
        representation_id: RepresentationId,
        /// Whether this was an input or output edge.
        edge: ArtifactEdgeKind,
        /// Fingerprint algorithm.
        algorithm: String,
        /// Fingerprint algorithm version.
        version: u16,
        /// Value captured on the activity edge.
        snapshot_value: Vec<u8>,
        /// Current representation fingerprint value.
        current_value: Vec<u8>,
    },
    /// A resource observation changed and its representation fingerprint has
    /// not yet been recomputed.
    FingerprintRecomputationPending {
        /// Activity owning the edge.
        activity_id: ActivityId,
        /// Representation whose aggregate fingerprint is dirty.
        representation_id: RepresentationId,
        /// Whether this was an input or output edge.
        edge: ArtifactEdgeKind,
    },
    /// An upstream produced representation is not current.
    UpstreamNotCurrent {
        /// Immediate input that depends on the upstream result.
        representation_id: RepresentationId,
        /// State computed for that upstream representation.
        state: ArtifactKnowledgeState,
    },
    /// Evaluation stopped at an explicit resource bound.
    TraversalTruncated {
        /// Bound that was reached.
        limit: ArtifactTraversalLimitKind,
        /// Representation at which traversal stopped.
        representation_id: RepresentationId,
    },
}

/// Explicit bounds for transitive artifact evaluation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ArtifactEvaluationLimits {
    max_depth: u32,
    max_representations: u32,
}

impl ArtifactEvaluationLimits {
    /// Conservative default suitable for interactive inspection.
    pub const DEFAULT: Self = Self {
        max_depth: 64,
        max_representations: 1_000,
    };

    /// Creates positive traversal bounds.
    ///
    /// # Errors
    ///
    /// Returns an invalid-argument error if either bound is zero or if the
    /// representation bound exceeds 100,000.
    pub fn new(max_depth: u32, max_representations: u32) -> crate::Result<Self> {
        if max_depth == 0 || max_representations == 0 || max_representations > 100_000 {
            return Err(crate::Error::new(
                crate::ErrorKind::InvalidArgument,
                "artifact evaluation bounds must be positive and visit at most 100000 representations",
            ));
        }
        Ok(Self {
            max_depth,
            max_representations,
        })
    }

    /// Maximum number of upstream activity edges followed from the target.
    #[must_use]
    pub const fn max_depth(self) -> u32 {
        self.max_depth
    }

    /// Maximum number of distinct representations inspected.
    #[must_use]
    pub const fn max_representations(self) -> u32 {
        self.max_representations
    }
}

impl Default for ArtifactEvaluationLimits {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// Complete, explainable result of evaluating one produced representation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArtifactEvaluation {
    representation_id: RepresentationId,
    state: ArtifactKnowledgeState,
    reasons: Vec<ArtifactKnowledgeReason>,
    visited_representations: u32,
    truncated: bool,
}

impl ArtifactEvaluation {
    /// Creates a backend-computed evaluation result.
    #[must_use]
    pub fn new(
        representation_id: RepresentationId,
        state: ArtifactKnowledgeState,
        reasons: Vec<ArtifactKnowledgeReason>,
        visited_representations: u32,
        truncated: bool,
    ) -> Self {
        Self {
            representation_id,
            state,
            reasons,
            visited_representations,
            truncated,
        }
    }

    /// Returns the representation that was evaluated.
    #[must_use]
    pub const fn representation_id(&self) -> RepresentationId {
        self.representation_id
    }

    /// Returns the derived knowledge state.
    #[must_use]
    pub const fn state(&self) -> ArtifactKnowledgeState {
        self.state
    }

    /// Returns stable, user-explainable reasons for a non-current result.
    #[must_use]
    pub fn reasons(&self) -> &[ArtifactKnowledgeReason] {
        &self.reasons
    }

    /// Returns how many distinct representations were inspected.
    #[must_use]
    pub const fn visited_representations(&self) -> u32 {
        self.visited_representations
    }

    /// Returns whether an explicit traversal bound stopped evaluation.
    #[must_use]
    pub const fn is_truncated(&self) -> bool {
        self.truncated
    }
}

/// One missing condition that prevents an artifact from being reproducible.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum ArtifactReproducibilityIssue {
    /// No activity records how the representation was produced.
    ProducingActivityMissing,
    /// More than one activity claims to have produced the representation.
    ProducingActivityAmbiguous {
        /// Number of producing activities found.
        activity_count: u32,
    },
    /// The producing activity does not identify the tool that performed it.
    ToolIdentityMissing {
        /// Producing activity missing the condition.
        activity_id: ActivityId,
    },
    /// The producing activity has no recorded parameter metadata.
    ParametersMissing {
        /// Producing activity missing the condition.
        activity_id: ActivityId,
    },
    /// An input representation referenced by the activity is absent.
    InputRepresentationMissing {
        /// Producing activity referencing the input.
        activity_id: ActivityId,
        /// Missing input representation.
        representation_id: RepresentationId,
    },
}

/// Structured answer to whether production knowledge can reproduce an artifact.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArtifactReproducibilityReport {
    representation_id: RepresentationId,
    producing_activity_id: Option<ActivityId>,
    activity_kind: Option<ActivityKind>,
    issues: Vec<ArtifactReproducibilityIssue>,
}

impl ArtifactReproducibilityReport {
    /// Creates a backend-computed reproducibility report.
    #[must_use]
    pub fn new(
        representation_id: RepresentationId,
        producing_activity_id: Option<ActivityId>,
        activity_kind: Option<ActivityKind>,
        issues: Vec<ArtifactReproducibilityIssue>,
    ) -> Self {
        Self {
            representation_id,
            producing_activity_id,
            activity_kind,
            issues,
        }
    }

    /// Returns the artifact representation being described.
    #[must_use]
    pub const fn representation_id(&self) -> RepresentationId {
        self.representation_id
    }

    /// Returns the single producing activity, when one exists unambiguously.
    #[must_use]
    pub const fn producing_activity_id(&self) -> Option<ActivityId> {
        self.producing_activity_id
    }

    /// Returns the recorded open-world activity kind, when unambiguous.
    #[must_use]
    pub const fn activity_kind(&self) -> Option<&ActivityKind> {
        self.activity_kind.as_ref()
    }

    /// Returns every missing reproducibility condition.
    #[must_use]
    pub fn issues(&self) -> &[ArtifactReproducibilityIssue] {
        &self.issues
    }

    /// Returns whether all reproducibility conditions are recorded.
    #[must_use]
    pub fn is_reproducible(&self) -> bool {
        self.issues.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn evaluation_limits_are_explicit_and_bounded() {
        assert!(ArtifactEvaluationLimits::new(0, 1).is_err());
        assert!(ArtifactEvaluationLimits::new(1, 0).is_err());
        assert!(ArtifactEvaluationLimits::new(1, 100_001).is_err());
        let limits = ArtifactEvaluationLimits::new(50, 10_000).expect("valid limits");
        assert_eq!(limits.max_depth(), 50);
        assert_eq!(limits.max_representations(), 10_000);
    }
}
