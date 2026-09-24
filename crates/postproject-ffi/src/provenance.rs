//! C-ABI-owned projection of production provenance activities.

use std::ffi::CString;

use postproject_core::{Activity, ActivityEdgeSnapshot, ActivityId, Error, RepresentationId};

use crate::exact_cstring;

/// Opaque immutable activity result set owned by the C caller.
pub struct PpActivitySet {
    pub(crate) activities: Vec<AbiActivity>,
}

pub(crate) struct AbiActivity {
    pub(crate) id: ActivityId,
    pub(crate) kind: CString,
    pub(crate) started_at_unix_micros: Option<i64>,
    pub(crate) finished_at_unix_micros: Option<i64>,
    pub(crate) tool: Option<AbiTool>,
    pub(crate) agent: Option<AbiAgent>,
    pub(crate) inputs: Vec<AbiActivityEdge>,
    pub(crate) outputs: Vec<AbiActivityEdge>,
}

pub(crate) struct AbiActivityEdge {
    pub(crate) representation_id: RepresentationId,
    pub(crate) role: Option<CString>,
    pub(crate) snapshot: Option<AbiActivityEdgeSnapshot>,
}

pub(crate) struct AbiActivityEdgeSnapshot {
    pub(crate) revision_sequence: u64,
    pub(crate) fingerprints: Vec<AbiFingerprintSnapshot>,
}

pub(crate) struct AbiFingerprintSnapshot {
    pub(crate) algorithm: CString,
    pub(crate) version: u16,
    pub(crate) value: Vec<u8>,
    pub(crate) observed_revision_sequence: Option<u64>,
}

pub(crate) struct AbiTool {
    pub(crate) name: CString,
    pub(crate) version: Option<CString>,
    pub(crate) uri: Option<CString>,
}

pub(crate) struct AbiAgent {
    pub(crate) name: Option<CString>,
    pub(crate) identifier_scheme: Option<CString>,
    pub(crate) identifier_value: Option<CString>,
    pub(crate) identifier_qualifier: Option<CString>,
}

impl PpActivitySet {
    pub(crate) fn new(activities: &[Activity]) -> Result<Self, Error> {
        let activities = activities
            .iter()
            .map(AbiActivity::try_from)
            .collect::<Result<_, _>>()?;
        Ok(Self { activities })
    }
}

impl TryFrom<&Activity> for AbiActivity {
    type Error = Error;

    fn try_from(activity: &Activity) -> Result<Self, Self::Error> {
        Ok(Self {
            id: activity.id(),
            kind: exact_cstring(activity.kind().as_str(), "activity kind")?,
            started_at_unix_micros: activity
                .started_at()
                .map(postproject_core::Timestamp::as_unix_micros),
            finished_at_unix_micros: activity
                .finished_at()
                .map(postproject_core::Timestamp::as_unix_micros),
            tool: activity.tool().map(AbiTool::try_from).transpose()?,
            agent: activity.agent().map(AbiAgent::try_from).transpose()?,
            inputs: activity
                .inputs()
                .iter()
                .map(|input| {
                    AbiActivityEdge::new(
                        input.representation_id(),
                        input.role().map(postproject_core::ActivityRole::as_str),
                        input.snapshot(),
                    )
                })
                .collect::<Result<_, _>>()?,
            outputs: activity
                .outputs()
                .iter()
                .map(|output| {
                    AbiActivityEdge::new(
                        output.representation_id(),
                        output.role().map(postproject_core::ActivityRole::as_str),
                        output.snapshot(),
                    )
                })
                .collect::<Result<_, _>>()?,
        })
    }
}

impl AbiActivityEdge {
    fn new(
        representation_id: RepresentationId,
        role: Option<&str>,
        snapshot: Option<&ActivityEdgeSnapshot>,
    ) -> Result<Self, Error> {
        Ok(Self {
            representation_id,
            role: role
                .map(|value| exact_cstring(value, "activity edge role"))
                .transpose()?,
            snapshot: snapshot
                .map(AbiActivityEdgeSnapshot::try_from)
                .transpose()?,
        })
    }
}

impl TryFrom<&ActivityEdgeSnapshot> for AbiActivityEdgeSnapshot {
    type Error = Error;

    fn try_from(snapshot: &ActivityEdgeSnapshot) -> Result<Self, Self::Error> {
        let fingerprints = snapshot
            .fingerprints()
            .iter()
            .map(|fingerprint| {
                Ok(AbiFingerprintSnapshot {
                    algorithm: exact_cstring(
                        fingerprint.algorithm(),
                        "activity snapshot fingerprint algorithm",
                    )?,
                    version: fingerprint.version(),
                    value: fingerprint.value().to_vec(),
                    observed_revision_sequence: fingerprint.observed_revision_sequence(),
                })
            })
            .collect::<Result<_, Error>>()?;
        Ok(Self {
            revision_sequence: snapshot.revision_sequence(),
            fingerprints,
        })
    }
}

impl TryFrom<&postproject_core::ToolIdentity> for AbiTool {
    type Error = Error;

    fn try_from(tool: &postproject_core::ToolIdentity) -> Result<Self, Self::Error> {
        Ok(Self {
            name: exact_cstring(tool.name(), "activity tool name")?,
            version: tool
                .version()
                .map(|value| exact_cstring(value, "activity tool version"))
                .transpose()?,
            uri: tool
                .uri()
                .map(|value| exact_cstring(value, "activity tool URI"))
                .transpose()?,
        })
    }
}

impl TryFrom<&postproject_core::AgentIdentity> for AbiAgent {
    type Error = Error;

    fn try_from(agent: &postproject_core::AgentIdentity) -> Result<Self, Self::Error> {
        let identifier = agent.identifier();
        Ok(Self {
            name: agent
                .name()
                .map(|value| exact_cstring(value, "activity agent name"))
                .transpose()?,
            identifier_scheme: identifier
                .map(|value| exact_cstring(value.scheme().as_str(), "agent identifier scheme"))
                .transpose()?,
            identifier_value: identifier
                .map(|value| exact_cstring(value.value(), "agent identifier value"))
                .transpose()?,
            identifier_qualifier: identifier
                .and_then(postproject_core::ExternalIdentifier::qualifier)
                .map(|value| exact_cstring(value, "agent identifier qualifier"))
                .transpose()?,
        })
    }
}
