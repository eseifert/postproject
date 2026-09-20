//! C-ABI-owned projection of production provenance activities.

use std::ffi::CString;

use postproject_core::{Activity, ActivityId, Error};

use crate::{exact_cstring, length_as_u64};

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
    pub(crate) input_count: u64,
    pub(crate) output_count: u64,
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
            input_count: length_as_u64(activity.inputs().len())?,
            output_count: length_as_u64(activity.outputs().len())?,
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
