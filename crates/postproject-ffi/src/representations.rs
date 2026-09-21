use std::{ffi::CString, os::raw::c_char, ptr};

use postproject_core::{
    AssetId, ContentStructure, ContentStructureKind, Error, ErrorKind, Representation,
    RepresentationId, RepresentationKind, ResourceId,
};

use crate::{
    PpError, PpProduction, PpUuid, exact_cstring, ffi_call, initialize_const_output,
    initialize_output, initialize_uuid, initialize_value, item_at, require_output,
};

const PP_REPRESENTATION_ORIGINAL: u32 = 1;
const PP_REPRESENTATION_PROXY: u32 = 2;
const PP_REPRESENTATION_OPTIMIZED: u32 = 3;
const PP_REPRESENTATION_DERIVED: u32 = 4;

const PP_CONTENT_SINGLE_RESOURCE: u32 = 1;
const PP_CONTENT_IMAGE_SEQUENCE: u32 = 2;
const PP_CONTENT_ORDERED_PARTS: u32 = 3;
const PP_CONTENT_PACKAGE: u32 = 4;

/// Opaque immutable representation result set owned by the C caller.
pub struct PpRepresentationSet {
    representations: Vec<AbiRepresentation>,
}

struct AbiRepresentation {
    id: RepresentationId,
    asset_id: AssetId,
    kind: u32,
    structure_kind: u32,
    members: Vec<AbiMember>,
}

struct AbiMember {
    resource_id: ResourceId,
    role: Option<CString>,
    required: bool,
}

/// Loads the representations belonging to one asset in stable order.
///
/// # Safety
///
/// All input pointers must be live and readable. `out_representations` must be
/// writable. `out_error` may be null or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pp_production_representations(
    production: *const PpProduction,
    asset_id: *const PpUuid,
    out_representations: *mut *mut PpRepresentationSet,
    out_error: *mut *mut PpError,
) -> u32 {
    // SAFETY: Pointers are validated before use and outputs are initialized.
    unsafe {
        initialize_output(out_representations);
        ffi_call(out_error, || {
            let production = production
                .as_ref()
                .ok_or_else(|| invalid_argument("production must not be null"))?;
            let asset_id = asset_id
                .as_ref()
                .ok_or_else(|| invalid_argument("asset_id must not be null"))?;
            require_output(out_representations, "out_representations")?;
            let asset_id = AssetId::from_bytes(asset_id.bytes);
            let inner = production
                .state
                .inner
                .try_borrow()
                .map_err(|_| Error::new(ErrorKind::Conflict, "production is already in use"))?;
            if !inner.assets()?.iter().any(|asset| asset.id() == asset_id) {
                return Err(Error::new(ErrorKind::NotFound, "asset does not exist"));
            }
            let representations = inner
                .representations(asset_id)?
                .into_iter()
                .map(AbiRepresentation::try_from)
                .collect::<Result<Vec<_>, _>>()?;
            out_representations.write(Box::into_raw(Box::new(PpRepresentationSet {
                representations,
            })));
            Ok(())
        })
    }
}

/// Returns the number of representations. Null input returns zero.
///
/// # Safety
///
/// `representations` must be null or a live result-set handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pp_representation_set_count(
    representations: *const PpRepresentationSet,
) -> u64 {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        // SAFETY: A non-null handle is live by the caller contract.
        unsafe { representations.as_ref() }.map_or(0, |set| {
            u64::try_from(set.representations.len()).unwrap_or(u64::MAX)
        })
    }))
    .unwrap_or(0)
}

/// Reads one representation summary.
///
/// # Safety
///
/// The set must be live and every output pointer must be writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pp_representation_set_get(
    representations: *const PpRepresentationSet,
    index: u64,
    out_id: *mut PpUuid,
    out_asset_id: *mut PpUuid,
    out_kind: *mut u32,
    out_structure_kind: *mut u32,
    out_member_count: *mut u64,
    out_error: *mut *mut PpError,
) -> u32 {
    // SAFETY: Outputs are initialized and checked before writes.
    unsafe {
        initialize_uuid(out_id);
        initialize_uuid(out_asset_id);
        initialize_value(out_kind, 0);
        initialize_value(out_structure_kind, 0);
        initialize_value(out_member_count, 0);
        ffi_call(out_error, || {
            require_output(out_id, "out_id")?;
            require_output(out_asset_id, "out_asset_id")?;
            require_output(out_kind, "out_kind")?;
            require_output(out_structure_kind, "out_structure_kind")?;
            require_output(out_member_count, "out_member_count")?;
            let set = representations
                .as_ref()
                .ok_or_else(|| invalid_argument("representations must not be null"))?;
            let representation = item_at(&set.representations, index, "representation")?;
            out_id.write(PpUuid {
                bytes: representation.id.into_bytes(),
            });
            out_asset_id.write(PpUuid {
                bytes: representation.asset_id.into_bytes(),
            });
            out_kind.write(representation.kind);
            out_structure_kind.write(representation.structure_kind);
            out_member_count.write(u64::try_from(representation.members.len()).unwrap_or(u64::MAX));
            Ok(())
        })
    }
}

/// Reads one structural member. A null role means the structure has no role.
///
/// # Safety
///
/// The set must be live and every output pointer must be writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pp_representation_set_get_member(
    representations: *const PpRepresentationSet,
    representation_index: u64,
    member_index: u64,
    out_resource_id: *mut PpUuid,
    out_role: *mut *const c_char,
    out_required: *mut u8,
    out_error: *mut *mut PpError,
) -> u32 {
    // SAFETY: Outputs are initialized and checked before writes.
    unsafe {
        initialize_uuid(out_resource_id);
        initialize_const_output(out_role);
        initialize_value(out_required, 0);
        ffi_call(out_error, || {
            require_output(out_resource_id, "out_resource_id")?;
            require_output(out_role, "out_role")?;
            require_output(out_required, "out_required")?;
            let set = representations
                .as_ref()
                .ok_or_else(|| invalid_argument("representations must not be null"))?;
            let representation =
                item_at(&set.representations, representation_index, "representation")?;
            let member = item_at(&representation.members, member_index, "member")?;
            out_resource_id.write(PpUuid {
                bytes: member.resource_id.into_bytes(),
            });
            out_role.write(
                member
                    .role
                    .as_ref()
                    .map_or(ptr::null(), |role| role.as_ptr()),
            );
            out_required.write(u8::from(member.required));
            Ok(())
        })
    }
}

/// Releases a representation result set. Null is a no-op.
///
/// # Safety
///
/// A non-null pointer must be an unreleased result-set handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pp_representation_set_release(representations: *mut PpRepresentationSet) {
    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        if !representations.is_null() {
            // SAFETY: Ownership is transferred back exactly once by contract.
            drop(unsafe { Box::from_raw(representations) });
        }
    }));
}

impl TryFrom<Representation> for AbiRepresentation {
    type Error = Error;

    fn try_from(representation: Representation) -> Result<Self, Self::Error> {
        let structure = representation.content_structure();
        Ok(Self {
            id: representation.id(),
            asset_id: representation.asset_id(),
            kind: representation_kind(representation.kind()),
            structure_kind: content_structure_kind(structure.kind()),
            members: members(structure)?,
        })
    }
}

fn members(structure: &ContentStructure) -> Result<Vec<AbiMember>, Error> {
    if let Some(members) = structure.members() {
        return members
            .iter()
            .map(|member| {
                Ok(AbiMember {
                    resource_id: member.resource_id(),
                    role: Some(exact_cstring(member.role().as_str(), "resource role")?),
                    required: member.is_required(),
                })
            })
            .collect();
    }
    Ok(structure
        .resource_ids()
        .into_iter()
        .map(|resource_id| AbiMember {
            resource_id,
            role: None,
            required: true,
        })
        .collect())
}

const fn representation_kind(kind: RepresentationKind) -> u32 {
    match kind {
        RepresentationKind::Original => PP_REPRESENTATION_ORIGINAL,
        RepresentationKind::Proxy => PP_REPRESENTATION_PROXY,
        RepresentationKind::Optimized => PP_REPRESENTATION_OPTIMIZED,
        RepresentationKind::Derived => PP_REPRESENTATION_DERIVED,
        _ => 0,
    }
}

const fn content_structure_kind(kind: ContentStructureKind) -> u32 {
    match kind {
        ContentStructureKind::SingleResource => PP_CONTENT_SINGLE_RESOURCE,
        ContentStructureKind::ImageSequence => PP_CONTENT_IMAGE_SEQUENCE,
        ContentStructureKind::OrderedParts => PP_CONTENT_ORDERED_PARTS,
        ContentStructureKind::Package => PP_CONTENT_PACKAGE,
        _ => 0,
    }
}

fn invalid_argument(message: impl Into<String>) -> Error {
    Error::new(ErrorKind::InvalidArgument, message)
}
