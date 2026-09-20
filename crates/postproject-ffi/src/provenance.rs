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
    pub(crate) input_count: u64,
    pub(crate) output_count: u64,
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
            input_count: length_as_u64(activity.inputs().len())?,
            output_count: length_as_u64(activity.outputs().len())?,
        })
    }
}
