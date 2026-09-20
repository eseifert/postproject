//! C-ABI-owned projections of durable project revisions.

use std::ffi::CString;

use postproject_core::{Error, Revision, RevisionId, TransactionId};

use crate::exact_cstring;

/// Opaque immutable revision result set owned by the C caller.
pub struct PpRevisionSet {
    pub(crate) revisions: Vec<AbiRevision>,
}

pub(crate) struct AbiRevision {
    pub(crate) id: RevisionId,
    pub(crate) sequence: u64,
    pub(crate) transaction_id: TransactionId,
    pub(crate) committed_at_unix_micros: i64,
    pub(crate) origin_name: Option<CString>,
    pub(crate) origin_version: Option<CString>,
    pub(crate) origin_uri: Option<CString>,
    pub(crate) message: Option<CString>,
}

impl PpRevisionSet {
    pub(crate) fn new(revisions: &[Revision]) -> Result<Self, Error> {
        let revisions = revisions
            .iter()
            .map(AbiRevision::try_from)
            .collect::<Result<_, _>>()?;
        Ok(Self { revisions })
    }
}

impl TryFrom<&Revision> for AbiRevision {
    type Error = Error;

    fn try_from(revision: &Revision) -> Result<Self, Self::Error> {
        let origin = revision.origin();
        Ok(Self {
            id: revision.id(),
            sequence: revision.sequence(),
            transaction_id: revision.transaction_id(),
            committed_at_unix_micros: revision.committed_at().as_unix_micros(),
            origin_name: origin
                .map(|value| exact_cstring(value.name(), "revision origin name"))
                .transpose()?,
            origin_version: origin
                .and_then(postproject_core::OriginIdentity::version)
                .map(|value| exact_cstring(value, "revision origin version"))
                .transpose()?,
            origin_uri: origin
                .and_then(postproject_core::OriginIdentity::uri)
                .map(|value| exact_cstring(value, "revision origin URI"))
                .transpose()?,
            message: revision
                .message()
                .map(|value| exact_cstring(value, "revision message"))
                .transpose()?,
        })
    }
}
