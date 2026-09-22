"""Generated from include/postproject/postproject.h; do not edit manually."""

from __future__ import annotations

import ctypes


class Production(ctypes.Structure):
    pass


class Transaction(ctypes.Structure):
    pass


class RepresentationSet(ctypes.Structure):
    pass


class ResolutionSet(ctypes.Structure):
    pass


class ExternalIdentifierSet(ctypes.Structure):
    pass


class ObjectRefSet(ctypes.Structure):
    pass


class MetadataSet(ctypes.Structure):
    pass


class MetadataValue(ctypes.Structure):
    pass


class MetadataInput(ctypes.Structure):
    pass


class ActivitySet(ctypes.Structure):
    pass


class RevisionSet(ctypes.Structure):
    pass


class RevisionEventSet(ctypes.Structure):
    pass


class Error(ctypes.Structure):
    pass


class Uuid(ctypes.Structure):
    pass


class ObjectRef(ctypes.Structure):
    pass


class RevisionEvent(ctypes.Structure):
    pass


class ActivityEdge(ctypes.Structure):
    pass


ObjectKind = ctypes.c_uint32
RepresentationKind = ctypes.c_uint32
ContentStructureKind = ctypes.c_uint32
LocatorAvailability = ctypes.c_uint32
RevisionEventKind = ctypes.c_uint32
MetadataValueKind = ctypes.c_uint32
ErrorCode = ctypes.c_uint32
RepresentationAvailability = ctypes.c_uint32
ResourceResolutionState = ctypes.c_uint32
AvailabilityIssueKind = ctypes.c_uint32
EvidenceKind = ctypes.c_uint32


PP_OBJECT_PRODUCTION = 1
PP_OBJECT_ASSET = 2
PP_OBJECT_REPRESENTATION = 3
PP_OBJECT_RESOURCE = 4
PP_OBJECT_ACTIVITY = 5
PP_REPRESENTATION_ORIGINAL = 1
PP_REPRESENTATION_PROXY = 2
PP_REPRESENTATION_OPTIMIZED = 3
PP_REPRESENTATION_DERIVED = 4
PP_CONTENT_SINGLE_RESOURCE = 1
PP_CONTENT_IMAGE_SEQUENCE = 2
PP_CONTENT_ORDERED_PARTS = 3
PP_CONTENT_PACKAGE = 4
PP_LOCATOR_UNKNOWN = 1
PP_LOCATOR_ONLINE = 2
PP_LOCATOR_OFFLINE = 3
PP_REVISION_ASSET_IMPORTED = 1
PP_REVISION_REPRESENTATION_ADDED = 2
PP_REVISION_RESOURCE_ADDED = 3
PP_REVISION_REPRESENTATION_RESOURCE_ADDED = 4
PP_REVISION_LOCATOR_ADDED = 5
PP_REVISION_MEDIA_ROOT_ADDED = 6
PP_REVISION_EXTERNAL_IDENTIFIER_ADDED = 7
PP_REVISION_EXTERNAL_IDENTIFIER_REMOVED = 8
PP_REVISION_METADATA_ADDED_OR_REPLACED = 9
PP_REVISION_METADATA_REMOVED = 10
PP_REVISION_ACTIVITY_CREATED = 11
PP_REVISION_ACTIVITY_INPUT_ADDED = 12
PP_REVISION_ACTIVITY_OUTPUT_ADDED = 13
PP_METADATA_STRING = 1
PP_METADATA_LANG_STRING = 2
PP_METADATA_I64 = 3
PP_METADATA_U64 = 4
PP_METADATA_DECIMAL = 5
PP_METADATA_BOOL = 6
PP_METADATA_TIMESTAMP = 7
PP_METADATA_URI = 8
PP_METADATA_BYTES = 9
PP_METADATA_RATIONAL = 10
PP_METADATA_LIST = 11
PP_METADATA_STRUCT = 12
PP_METADATA_REFERENCE = 13
PP_OK = 0
PP_ERROR_INVALID_ARGUMENT = 1
PP_ERROR_NOT_FOUND = 2
PP_ERROR_ALREADY_EXISTS = 3
PP_ERROR_IO = 4
PP_ERROR_STORAGE = 5
PP_ERROR_MIGRATION = 6
PP_ERROR_CONFLICT = 7
PP_ERROR_AMBIGUOUS_RESOLUTION = 8
PP_ERROR_FINGERPRINT = 9
PP_ERROR_UNSUPPORTED = 10
PP_ERROR_INTERNAL = 255
PP_AVAILABILITY_ONLINE = 1
PP_AVAILABILITY_PARTIAL = 2
PP_AVAILABILITY_OFFLINE = 3
PP_AVAILABILITY_AMBIGUOUS = 4
PP_AVAILABILITY_ERROR = 5
PP_RESOURCE_ONLINE_AT_KNOWN_LOCATOR = 1
PP_RESOURCE_RESOLVED_EXACT = 2
PP_RESOURCE_RESOLVED_PROBABLE = 3
PP_RESOURCE_OFFLINE = 4
PP_RESOURCE_AMBIGUOUS = 5
PP_RESOURCE_RESOLUTION_ERROR = 6
PP_AVAILABILITY_ISSUE_OFFLINE_RESOURCE = 1
PP_AVAILABILITY_ISSUE_AMBIGUOUS_RESOURCE = 2
PP_AVAILABILITY_ISSUE_RESOURCE_ERROR = 3
PP_AVAILABILITY_ISSUE_MISSING_FRAMES = 4
PP_EVIDENCE_KNOWN_LOCATOR_AVAILABLE = 1
PP_EVIDENCE_EXACT_FINGERPRINT_MATCH = 2
PP_EVIDENCE_FULL_HASH_MATCH = 3
PP_EVIDENCE_PARTIAL_FINGERPRINT_MATCH = 4
PP_EVIDENCE_FILE_SIZE_MATCH = 5
PP_EVIDENCE_FILE_NAME_MATCH = 6
PP_EVIDENCE_RELATIVE_PATH_SIMILARITY = 7
PP_EVIDENCE_MEDIA_ROOT_RELATION = 8
PP_EVIDENCE_CONFLICTING_CANDIDATE = 9
PP_EVIDENCE_DISCOVERY_ERROR = 10


Uuid._fields_ = [
    ("bytes", ctypes.c_uint8 * 16),
]

ObjectRef._fields_ = [
    ("kind", ObjectKind),
    ("id", Uuid),
]

RevisionEvent._fields_ = [
    ("kind", RevisionEventKind),
    ("position", ctypes.c_uint32),
    ("asset_id", Uuid),
    ("representation_id", Uuid),
    ("resource_id", Uuid),
    ("locator_id", Uuid),
    ("media_root_id", Uuid),
    ("activity_id", Uuid),
    ("target", ObjectRef),
    ("structural_position", ctypes.c_uint32),
    ("identifier_scheme", ctypes.c_char_p),
    ("identifier_value", ctypes.c_char_p),
    ("identifier_qualifier", ctypes.c_char_p),
    ("vocabulary", ctypes.c_char_p),
    ("property", ctypes.c_char_p),
    ("activity_kind", ctypes.c_char_p),
    ("role", ctypes.c_char_p),
]

ActivityEdge._fields_ = [
    ("representation_id", Uuid),
    ("role", ctypes.c_char_p),
]


PUBLIC_STRUCTS = {
    "pp_uuid_t": (Uuid, ("bytes",)), 
    "pp_object_ref_t": (ObjectRef, ("kind", "id")), 
    "pp_revision_event_t": (RevisionEvent, ("kind", "position", "asset_id", "representation_id", "resource_id", "locator_id", "media_root_id", "activity_id", "target", "structural_position", "identifier_scheme", "identifier_value", "identifier_qualifier", "vocabulary", "property", "activity_kind", "role")), 
    "pp_activity_edge_t": (ActivityEdge, ("representation_id", "role")), 
}


EXPORTED_SYMBOLS = (
    "pp_abi_version",
    "pp_activity_set_count",
    "pp_activity_set_get",
    "pp_activity_set_get_agent",
    "pp_activity_set_get_input",
    "pp_activity_set_get_output",
    "pp_activity_set_get_tool",
    "pp_activity_set_release",
    "pp_error_code",
    "pp_error_message",
    "pp_error_release",
    "pp_external_identifier_set_count",
    "pp_external_identifier_set_get",
    "pp_external_identifier_set_release",
    "pp_host_binding_format",
    "pp_host_binding_parse",
    "pp_host_binding_release",
    "pp_metadata_input_create_bool",
    "pp_metadata_input_create_bytes",
    "pp_metadata_input_create_decimal",
    "pp_metadata_input_create_i64",
    "pp_metadata_input_create_list",
    "pp_metadata_input_create_rational",
    "pp_metadata_input_create_reference",
    "pp_metadata_input_create_string",
    "pp_metadata_input_create_struct",
    "pp_metadata_input_create_timestamp",
    "pp_metadata_input_create_u64",
    "pp_metadata_input_create_uri",
    "pp_metadata_input_release",
    "pp_metadata_set_count",
    "pp_metadata_set_get",
    "pp_metadata_set_release",
    "pp_metadata_value_get_bool",
    "pp_metadata_value_get_bytes",
    "pp_metadata_value_get_decimal",
    "pp_metadata_value_get_i64",
    "pp_metadata_value_get_rational",
    "pp_metadata_value_get_reference",
    "pp_metadata_value_get_string",
    "pp_metadata_value_get_timestamp",
    "pp_metadata_value_get_u64",
    "pp_metadata_value_get_uri",
    "pp_metadata_value_kind",
    "pp_metadata_value_list_count",
    "pp_metadata_value_list_get",
    "pp_metadata_value_struct_count",
    "pp_metadata_value_struct_get",
    "pp_object_ref_set_count",
    "pp_object_ref_set_get",
    "pp_object_ref_set_release",
    "pp_production_activities",
    "pp_production_activities_consuming",
    "pp_production_activities_producing",
    "pp_production_asset_exists",
    "pp_production_begin_transaction",
    "pp_production_changes_since",
    "pp_production_create",
    "pp_production_external_identifiers",
    "pp_production_find_by_external_identifier",
    "pp_production_find_metadata",
    "pp_production_id",
    "pp_production_latest_revision",
    "pp_production_metadata",
    "pp_production_open",
    "pp_production_provenance_ancestors",
    "pp_production_provenance_descendants",
    "pp_production_release",
    "pp_production_representations",
    "pp_production_resolve_asset",
    "pp_production_revision_events",
    "pp_representation_set_count",
    "pp_representation_set_get",
    "pp_representation_set_get_fingerprint",
    "pp_representation_set_get_locator",
    "pp_representation_set_get_member",
    "pp_representation_set_get_resource",
    "pp_representation_set_get_resource_fingerprint",
    "pp_representation_set_get_sequence",
    "pp_representation_set_get_sequence_missing_frame",
    "pp_representation_set_release",
    "pp_resolution_set_get_candidate",
    "pp_resolution_set_get_candidate_evidence",
    "pp_resolution_set_get_issue",
    "pp_resolution_set_get_issue_frame",
    "pp_resolution_set_get_representation",
    "pp_resolution_set_get_resource",
    "pp_resolution_set_get_resource_evidence",
    "pp_resolution_set_release",
    "pp_resolution_set_representation_count",
    "pp_revision_event_set_count",
    "pp_revision_event_set_get",
    "pp_revision_event_set_release",
    "pp_revision_set_count",
    "pp_revision_set_get",
    "pp_revision_set_release",
    "pp_transaction_add_external_identifier",
    "pp_transaction_add_image_sequence_representation",
    "pp_transaction_add_media_root",
    "pp_transaction_add_metadata_value",
    "pp_transaction_add_single_file_representation",
    "pp_transaction_commit",
    "pp_transaction_confirm_locator",
    "pp_transaction_create_activity",
    "pp_transaction_import_media",
    "pp_transaction_release",
    "pp_transaction_remove_external_identifier",
    "pp_transaction_remove_metadata_property",
    "pp_transaction_rollback",
    "pp_transaction_set_revision_context",
)


def configure_api(lib: ctypes.CDLL) -> None:
    """Configure every function declared by the public C header."""

    lib.pp_abi_version.argtypes = []
    lib.pp_abi_version.restype = ctypes.c_uint32
    lib.pp_host_binding_format.argtypes = [ctypes.POINTER(Uuid), ctypes.POINTER(ObjectRef), ctypes.POINTER(ctypes.c_char_p), ctypes.POINTER(ctypes.POINTER(Error))]
    lib.pp_host_binding_format.restype = ErrorCode
    lib.pp_host_binding_parse.argtypes = [ctypes.c_char_p, ctypes.POINTER(Uuid), ctypes.POINTER(ObjectRef), ctypes.POINTER(ctypes.POINTER(Error))]
    lib.pp_host_binding_parse.restype = ErrorCode
    lib.pp_host_binding_release.argtypes = [ctypes.c_char_p]
    lib.pp_host_binding_release.restype = None
    lib.pp_production_create.argtypes = [ctypes.c_char_p, ctypes.c_char_p, ctypes.POINTER(ctypes.POINTER(Production)), ctypes.POINTER(ctypes.POINTER(Error))]
    lib.pp_production_create.restype = ErrorCode
    lib.pp_production_open.argtypes = [ctypes.c_char_p, ctypes.POINTER(ctypes.POINTER(Production)), ctypes.POINTER(ctypes.POINTER(Error))]
    lib.pp_production_open.restype = ErrorCode
    lib.pp_production_id.argtypes = [ctypes.POINTER(Production), ctypes.POINTER(Uuid), ctypes.POINTER(ctypes.POINTER(Error))]
    lib.pp_production_id.restype = ErrorCode
    lib.pp_production_asset_exists.argtypes = [ctypes.POINTER(Production), ctypes.POINTER(Uuid), ctypes.POINTER(ctypes.c_uint8), ctypes.POINTER(ctypes.POINTER(Error))]
    lib.pp_production_asset_exists.restype = ErrorCode
    lib.pp_production_representations.argtypes = [ctypes.POINTER(Production), ctypes.POINTER(Uuid), ctypes.POINTER(ctypes.POINTER(RepresentationSet)), ctypes.POINTER(ctypes.POINTER(Error))]
    lib.pp_production_representations.restype = ErrorCode
    lib.pp_representation_set_count.argtypes = [ctypes.POINTER(RepresentationSet)]
    lib.pp_representation_set_count.restype = ctypes.c_uint64
    lib.pp_representation_set_get.argtypes = [ctypes.POINTER(RepresentationSet), ctypes.c_uint64, ctypes.POINTER(Uuid), ctypes.POINTER(Uuid), ctypes.POINTER(RepresentationKind), ctypes.POINTER(ContentStructureKind), ctypes.POINTER(ctypes.c_uint64), ctypes.POINTER(ctypes.c_uint64), ctypes.POINTER(ctypes.c_uint64), ctypes.POINTER(ctypes.POINTER(Error))]
    lib.pp_representation_set_get.restype = ErrorCode
    lib.pp_representation_set_get_fingerprint.argtypes = [ctypes.POINTER(RepresentationSet), ctypes.c_uint64, ctypes.c_uint64, ctypes.POINTER(ctypes.c_char_p), ctypes.POINTER(ctypes.c_uint16), ctypes.POINTER(ctypes.POINTER(ctypes.c_uint8)), ctypes.POINTER(ctypes.c_uint64), ctypes.POINTER(ctypes.POINTER(Error))]
    lib.pp_representation_set_get_fingerprint.restype = ErrorCode
    lib.pp_representation_set_get_member.argtypes = [ctypes.POINTER(RepresentationSet), ctypes.c_uint64, ctypes.c_uint64, ctypes.POINTER(Uuid), ctypes.POINTER(ctypes.c_char_p), ctypes.POINTER(ctypes.c_uint8), ctypes.POINTER(ctypes.POINTER(Error))]
    lib.pp_representation_set_get_member.restype = ErrorCode
    lib.pp_representation_set_get_sequence.argtypes = [ctypes.POINTER(RepresentationSet), ctypes.c_uint64, ctypes.POINTER(ctypes.c_char_p), ctypes.POINTER(ctypes.c_char_p), ctypes.POINTER(ctypes.c_uint8), ctypes.POINTER(ctypes.c_int64), ctypes.POINTER(ctypes.c_int64), ctypes.POINTER(ctypes.c_uint32), ctypes.POINTER(ctypes.c_uint32), ctypes.POINTER(ctypes.c_uint32), ctypes.POINTER(ctypes.c_uint64), ctypes.POINTER(ctypes.POINTER(Error))]
    lib.pp_representation_set_get_sequence.restype = ErrorCode
    lib.pp_representation_set_get_sequence_missing_frame.argtypes = [ctypes.POINTER(RepresentationSet), ctypes.c_uint64, ctypes.c_uint64, ctypes.POINTER(ctypes.c_int64), ctypes.POINTER(ctypes.POINTER(Error))]
    lib.pp_representation_set_get_sequence_missing_frame.restype = ErrorCode
    lib.pp_representation_set_get_resource.argtypes = [ctypes.POINTER(RepresentationSet), ctypes.c_uint64, ctypes.c_uint64, ctypes.POINTER(Uuid), ctypes.POINTER(ctypes.c_uint8), ctypes.POINTER(ctypes.c_uint64), ctypes.POINTER(ctypes.c_uint8), ctypes.POINTER(ctypes.c_int64), ctypes.POINTER(ctypes.c_uint64), ctypes.POINTER(ctypes.c_uint64), ctypes.POINTER(ctypes.POINTER(Error))]
    lib.pp_representation_set_get_resource.restype = ErrorCode
    lib.pp_representation_set_get_resource_fingerprint.argtypes = [ctypes.POINTER(RepresentationSet), ctypes.c_uint64, ctypes.c_uint64, ctypes.c_uint64, ctypes.POINTER(ctypes.c_char_p), ctypes.POINTER(ctypes.c_uint16), ctypes.POINTER(ctypes.POINTER(ctypes.c_uint8)), ctypes.POINTER(ctypes.c_uint64), ctypes.POINTER(ctypes.POINTER(Error))]
    lib.pp_representation_set_get_resource_fingerprint.restype = ErrorCode
    lib.pp_representation_set_get_locator.argtypes = [ctypes.POINTER(RepresentationSet), ctypes.c_uint64, ctypes.c_uint64, ctypes.c_uint64, ctypes.POINTER(Uuid), ctypes.POINTER(ctypes.c_char_p), ctypes.POINTER(LocatorAvailability), ctypes.POINTER(ctypes.c_uint8), ctypes.POINTER(ctypes.c_int64), ctypes.POINTER(ctypes.POINTER(Error))]
    lib.pp_representation_set_get_locator.restype = ErrorCode
    lib.pp_representation_set_release.argtypes = [ctypes.POINTER(RepresentationSet)]
    lib.pp_representation_set_release.restype = None
    lib.pp_production_external_identifiers.argtypes = [ctypes.POINTER(Production), ctypes.POINTER(ObjectRef), ctypes.POINTER(ctypes.POINTER(ExternalIdentifierSet)), ctypes.POINTER(ctypes.POINTER(Error))]
    lib.pp_production_external_identifiers.restype = ErrorCode
    lib.pp_production_find_by_external_identifier.argtypes = [ctypes.POINTER(Production), ctypes.c_char_p, ctypes.c_char_p, ctypes.POINTER(ctypes.POINTER(ObjectRefSet)), ctypes.POINTER(ctypes.POINTER(Error))]
    lib.pp_production_find_by_external_identifier.restype = ErrorCode
    lib.pp_external_identifier_set_count.argtypes = [ctypes.POINTER(ExternalIdentifierSet)]
    lib.pp_external_identifier_set_count.restype = ctypes.c_uint64
    lib.pp_external_identifier_set_get.argtypes = [ctypes.POINTER(ExternalIdentifierSet), ctypes.c_uint64, ctypes.POINTER(ctypes.c_char_p), ctypes.POINTER(ctypes.c_char_p), ctypes.POINTER(ctypes.c_char_p), ctypes.POINTER(ctypes.POINTER(Error))]
    lib.pp_external_identifier_set_get.restype = ErrorCode
    lib.pp_external_identifier_set_release.argtypes = [ctypes.POINTER(ExternalIdentifierSet)]
    lib.pp_external_identifier_set_release.restype = None
    lib.pp_object_ref_set_count.argtypes = [ctypes.POINTER(ObjectRefSet)]
    lib.pp_object_ref_set_count.restype = ctypes.c_uint64
    lib.pp_object_ref_set_get.argtypes = [ctypes.POINTER(ObjectRefSet), ctypes.c_uint64, ctypes.POINTER(ObjectRef), ctypes.POINTER(ctypes.POINTER(Error))]
    lib.pp_object_ref_set_get.restype = ErrorCode
    lib.pp_object_ref_set_release.argtypes = [ctypes.POINTER(ObjectRefSet)]
    lib.pp_object_ref_set_release.restype = None
    lib.pp_production_metadata.argtypes = [ctypes.POINTER(Production), ctypes.POINTER(ObjectRef), ctypes.POINTER(ctypes.POINTER(MetadataSet)), ctypes.POINTER(ctypes.POINTER(Error))]
    lib.pp_production_metadata.restype = ErrorCode
    lib.pp_production_find_metadata.argtypes = [ctypes.POINTER(Production), ctypes.c_char_p, ctypes.c_char_p, ctypes.POINTER(ctypes.POINTER(MetadataSet)), ctypes.POINTER(ctypes.POINTER(Error))]
    lib.pp_production_find_metadata.restype = ErrorCode
    lib.pp_metadata_set_count.argtypes = [ctypes.POINTER(MetadataSet)]
    lib.pp_metadata_set_count.restype = ctypes.c_uint64
    lib.pp_metadata_set_get.argtypes = [ctypes.POINTER(MetadataSet), ctypes.c_uint64, ctypes.POINTER(ObjectRef), ctypes.POINTER(ctypes.c_char_p), ctypes.POINTER(ctypes.c_char_p), ctypes.POINTER(ctypes.POINTER(MetadataValue)), ctypes.POINTER(ctypes.POINTER(Error))]
    lib.pp_metadata_set_get.restype = ErrorCode
    lib.pp_metadata_set_release.argtypes = [ctypes.POINTER(MetadataSet)]
    lib.pp_metadata_set_release.restype = None
    lib.pp_metadata_value_kind.argtypes = [ctypes.POINTER(MetadataValue)]
    lib.pp_metadata_value_kind.restype = MetadataValueKind
    lib.pp_metadata_value_get_string.argtypes = [ctypes.POINTER(MetadataValue), ctypes.POINTER(ctypes.c_char_p), ctypes.POINTER(ctypes.c_char_p), ctypes.POINTER(ctypes.POINTER(Error))]
    lib.pp_metadata_value_get_string.restype = ErrorCode
    lib.pp_metadata_value_get_i64.argtypes = [ctypes.POINTER(MetadataValue), ctypes.POINTER(ctypes.c_int64), ctypes.POINTER(ctypes.POINTER(Error))]
    lib.pp_metadata_value_get_i64.restype = ErrorCode
    lib.pp_metadata_value_get_u64.argtypes = [ctypes.POINTER(MetadataValue), ctypes.POINTER(ctypes.c_uint64), ctypes.POINTER(ctypes.POINTER(Error))]
    lib.pp_metadata_value_get_u64.restype = ErrorCode
    lib.pp_metadata_value_get_decimal.argtypes = [ctypes.POINTER(MetadataValue), ctypes.POINTER(ctypes.c_char_p), ctypes.POINTER(ctypes.c_uint32), ctypes.POINTER(ctypes.POINTER(Error))]
    lib.pp_metadata_value_get_decimal.restype = ErrorCode
    lib.pp_metadata_value_get_bool.argtypes = [ctypes.POINTER(MetadataValue), ctypes.POINTER(ctypes.c_uint8), ctypes.POINTER(ctypes.POINTER(Error))]
    lib.pp_metadata_value_get_bool.restype = ErrorCode
    lib.pp_metadata_value_get_timestamp.argtypes = [ctypes.POINTER(MetadataValue), ctypes.POINTER(ctypes.c_int64), ctypes.POINTER(ctypes.POINTER(Error))]
    lib.pp_metadata_value_get_timestamp.restype = ErrorCode
    lib.pp_metadata_value_get_uri.argtypes = [ctypes.POINTER(MetadataValue), ctypes.POINTER(ctypes.c_char_p), ctypes.POINTER(ctypes.POINTER(Error))]
    lib.pp_metadata_value_get_uri.restype = ErrorCode
    lib.pp_metadata_value_get_bytes.argtypes = [ctypes.POINTER(MetadataValue), ctypes.POINTER(ctypes.POINTER(ctypes.c_uint8)), ctypes.POINTER(ctypes.c_uint64), ctypes.POINTER(ctypes.POINTER(Error))]
    lib.pp_metadata_value_get_bytes.restype = ErrorCode
    lib.pp_metadata_value_get_rational.argtypes = [ctypes.POINTER(MetadataValue), ctypes.POINTER(ctypes.c_int64), ctypes.POINTER(ctypes.c_uint64), ctypes.POINTER(ctypes.POINTER(Error))]
    lib.pp_metadata_value_get_rational.restype = ErrorCode
    lib.pp_metadata_value_list_count.argtypes = [ctypes.POINTER(MetadataValue)]
    lib.pp_metadata_value_list_count.restype = ctypes.c_uint64
    lib.pp_metadata_value_list_get.argtypes = [ctypes.POINTER(MetadataValue), ctypes.c_uint64, ctypes.POINTER(ctypes.POINTER(MetadataValue)), ctypes.POINTER(ctypes.POINTER(Error))]
    lib.pp_metadata_value_list_get.restype = ErrorCode
    lib.pp_metadata_value_struct_count.argtypes = [ctypes.POINTER(MetadataValue)]
    lib.pp_metadata_value_struct_count.restype = ctypes.c_uint64
    lib.pp_metadata_value_struct_get.argtypes = [ctypes.POINTER(MetadataValue), ctypes.c_uint64, ctypes.POINTER(ctypes.c_char_p), ctypes.POINTER(ctypes.POINTER(MetadataValue)), ctypes.POINTER(ctypes.POINTER(Error))]
    lib.pp_metadata_value_struct_get.restype = ErrorCode
    lib.pp_metadata_value_get_reference.argtypes = [ctypes.POINTER(MetadataValue), ctypes.POINTER(ObjectRef), ctypes.POINTER(ctypes.POINTER(Error))]
    lib.pp_metadata_value_get_reference.restype = ErrorCode
    lib.pp_metadata_input_create_string.argtypes = [ctypes.c_char_p, ctypes.c_char_p, ctypes.POINTER(ctypes.POINTER(MetadataInput)), ctypes.POINTER(ctypes.POINTER(Error))]
    lib.pp_metadata_input_create_string.restype = ErrorCode
    lib.pp_metadata_input_create_i64.argtypes = [ctypes.c_int64, ctypes.POINTER(ctypes.POINTER(MetadataInput)), ctypes.POINTER(ctypes.POINTER(Error))]
    lib.pp_metadata_input_create_i64.restype = ErrorCode
    lib.pp_metadata_input_create_u64.argtypes = [ctypes.c_uint64, ctypes.POINTER(ctypes.POINTER(MetadataInput)), ctypes.POINTER(ctypes.POINTER(Error))]
    lib.pp_metadata_input_create_u64.restype = ErrorCode
    lib.pp_metadata_input_create_decimal.argtypes = [ctypes.c_char_p, ctypes.c_uint32, ctypes.POINTER(ctypes.POINTER(MetadataInput)), ctypes.POINTER(ctypes.POINTER(Error))]
    lib.pp_metadata_input_create_decimal.restype = ErrorCode
    lib.pp_metadata_input_create_bool.argtypes = [ctypes.c_uint8, ctypes.POINTER(ctypes.POINTER(MetadataInput)), ctypes.POINTER(ctypes.POINTER(Error))]
    lib.pp_metadata_input_create_bool.restype = ErrorCode
    lib.pp_metadata_input_create_timestamp.argtypes = [ctypes.c_int64, ctypes.POINTER(ctypes.POINTER(MetadataInput)), ctypes.POINTER(ctypes.POINTER(Error))]
    lib.pp_metadata_input_create_timestamp.restype = ErrorCode
    lib.pp_metadata_input_create_uri.argtypes = [ctypes.c_char_p, ctypes.POINTER(ctypes.POINTER(MetadataInput)), ctypes.POINTER(ctypes.POINTER(Error))]
    lib.pp_metadata_input_create_uri.restype = ErrorCode
    lib.pp_metadata_input_create_bytes.argtypes = [ctypes.POINTER(ctypes.c_uint8), ctypes.c_uint64, ctypes.POINTER(ctypes.POINTER(MetadataInput)), ctypes.POINTER(ctypes.POINTER(Error))]
    lib.pp_metadata_input_create_bytes.restype = ErrorCode
    lib.pp_metadata_input_create_rational.argtypes = [ctypes.c_int64, ctypes.c_uint64, ctypes.POINTER(ctypes.POINTER(MetadataInput)), ctypes.POINTER(ctypes.POINTER(Error))]
    lib.pp_metadata_input_create_rational.restype = ErrorCode
    lib.pp_metadata_input_create_reference.argtypes = [ctypes.POINTER(ObjectRef), ctypes.POINTER(ctypes.POINTER(MetadataInput)), ctypes.POINTER(ctypes.POINTER(Error))]
    lib.pp_metadata_input_create_reference.restype = ErrorCode
    lib.pp_metadata_input_create_list.argtypes = [ctypes.POINTER(ctypes.POINTER(MetadataInput)), ctypes.c_uint64, ctypes.POINTER(ctypes.POINTER(MetadataInput)), ctypes.POINTER(ctypes.POINTER(Error))]
    lib.pp_metadata_input_create_list.restype = ErrorCode
    lib.pp_metadata_input_create_struct.argtypes = [ctypes.POINTER(ctypes.c_char_p), ctypes.POINTER(ctypes.POINTER(MetadataInput)), ctypes.c_uint64, ctypes.POINTER(ctypes.POINTER(MetadataInput)), ctypes.POINTER(ctypes.POINTER(Error))]
    lib.pp_metadata_input_create_struct.restype = ErrorCode
    lib.pp_metadata_input_release.argtypes = [ctypes.POINTER(MetadataInput)]
    lib.pp_metadata_input_release.restype = None
    lib.pp_production_activities.argtypes = [ctypes.POINTER(Production), ctypes.POINTER(ctypes.POINTER(ActivitySet)), ctypes.POINTER(ctypes.POINTER(Error))]
    lib.pp_production_activities.restype = ErrorCode
    lib.pp_production_activities_producing.argtypes = [ctypes.POINTER(Production), ctypes.POINTER(Uuid), ctypes.POINTER(ctypes.POINTER(ActivitySet)), ctypes.POINTER(ctypes.POINTER(Error))]
    lib.pp_production_activities_producing.restype = ErrorCode
    lib.pp_production_activities_consuming.argtypes = [ctypes.POINTER(Production), ctypes.POINTER(Uuid), ctypes.POINTER(ctypes.POINTER(ActivitySet)), ctypes.POINTER(ctypes.POINTER(Error))]
    lib.pp_production_activities_consuming.restype = ErrorCode
    lib.pp_production_provenance_ancestors.argtypes = [ctypes.POINTER(Production), ctypes.POINTER(Uuid), ctypes.POINTER(ctypes.POINTER(ObjectRefSet)), ctypes.POINTER(ctypes.POINTER(Error))]
    lib.pp_production_provenance_ancestors.restype = ErrorCode
    lib.pp_production_provenance_descendants.argtypes = [ctypes.POINTER(Production), ctypes.POINTER(Uuid), ctypes.POINTER(ctypes.POINTER(ObjectRefSet)), ctypes.POINTER(ctypes.POINTER(Error))]
    lib.pp_production_provenance_descendants.restype = ErrorCode
    lib.pp_activity_set_count.argtypes = [ctypes.POINTER(ActivitySet)]
    lib.pp_activity_set_count.restype = ctypes.c_uint64
    lib.pp_activity_set_get.argtypes = [ctypes.POINTER(ActivitySet), ctypes.c_uint64, ctypes.POINTER(Uuid), ctypes.POINTER(ctypes.c_char_p), ctypes.POINTER(ctypes.c_uint8), ctypes.POINTER(ctypes.c_int64), ctypes.POINTER(ctypes.c_uint8), ctypes.POINTER(ctypes.c_int64), ctypes.POINTER(ctypes.c_uint64), ctypes.POINTER(ctypes.c_uint64), ctypes.POINTER(ctypes.POINTER(Error))]
    lib.pp_activity_set_get.restype = ErrorCode
    lib.pp_activity_set_get_tool.argtypes = [ctypes.POINTER(ActivitySet), ctypes.c_uint64, ctypes.POINTER(ctypes.c_char_p), ctypes.POINTER(ctypes.c_char_p), ctypes.POINTER(ctypes.c_char_p), ctypes.POINTER(ctypes.POINTER(Error))]
    lib.pp_activity_set_get_tool.restype = ErrorCode
    lib.pp_activity_set_get_agent.argtypes = [ctypes.POINTER(ActivitySet), ctypes.c_uint64, ctypes.POINTER(ctypes.c_char_p), ctypes.POINTER(ctypes.c_char_p), ctypes.POINTER(ctypes.c_char_p), ctypes.POINTER(ctypes.c_char_p), ctypes.POINTER(ctypes.POINTER(Error))]
    lib.pp_activity_set_get_agent.restype = ErrorCode
    lib.pp_activity_set_get_input.argtypes = [ctypes.POINTER(ActivitySet), ctypes.c_uint64, ctypes.c_uint64, ctypes.POINTER(Uuid), ctypes.POINTER(ctypes.c_char_p), ctypes.POINTER(ctypes.POINTER(Error))]
    lib.pp_activity_set_get_input.restype = ErrorCode
    lib.pp_activity_set_get_output.argtypes = [ctypes.POINTER(ActivitySet), ctypes.c_uint64, ctypes.c_uint64, ctypes.POINTER(Uuid), ctypes.POINTER(ctypes.c_char_p), ctypes.POINTER(ctypes.POINTER(Error))]
    lib.pp_activity_set_get_output.restype = ErrorCode
    lib.pp_activity_set_release.argtypes = [ctypes.POINTER(ActivitySet)]
    lib.pp_activity_set_release.restype = None
    lib.pp_production_latest_revision.argtypes = [ctypes.POINTER(Production), ctypes.POINTER(ctypes.POINTER(RevisionSet)), ctypes.POINTER(ctypes.POINTER(Error))]
    lib.pp_production_latest_revision.restype = ErrorCode
    lib.pp_production_changes_since.argtypes = [ctypes.POINTER(Production), ctypes.c_uint64, ctypes.c_uint32, ctypes.POINTER(ctypes.POINTER(RevisionSet)), ctypes.POINTER(ctypes.POINTER(Error))]
    lib.pp_production_changes_since.restype = ErrorCode
    lib.pp_revision_set_count.argtypes = [ctypes.POINTER(RevisionSet)]
    lib.pp_revision_set_count.restype = ctypes.c_uint64
    lib.pp_revision_set_get.argtypes = [ctypes.POINTER(RevisionSet), ctypes.c_uint64, ctypes.POINTER(Uuid), ctypes.POINTER(ctypes.c_uint64), ctypes.POINTER(Uuid), ctypes.POINTER(ctypes.c_int64), ctypes.POINTER(ctypes.c_char_p), ctypes.POINTER(ctypes.c_char_p), ctypes.POINTER(ctypes.c_char_p), ctypes.POINTER(ctypes.c_char_p), ctypes.POINTER(ctypes.POINTER(Error))]
    lib.pp_revision_set_get.restype = ErrorCode
    lib.pp_revision_set_release.argtypes = [ctypes.POINTER(RevisionSet)]
    lib.pp_revision_set_release.restype = None
    lib.pp_production_revision_events.argtypes = [ctypes.POINTER(Production), ctypes.POINTER(Uuid), ctypes.POINTER(ctypes.POINTER(RevisionEventSet)), ctypes.POINTER(ctypes.POINTER(Error))]
    lib.pp_production_revision_events.restype = ErrorCode
    lib.pp_revision_event_set_count.argtypes = [ctypes.POINTER(RevisionEventSet)]
    lib.pp_revision_event_set_count.restype = ctypes.c_uint64
    lib.pp_revision_event_set_get.argtypes = [ctypes.POINTER(RevisionEventSet), ctypes.c_uint64, ctypes.POINTER(RevisionEvent), ctypes.POINTER(ctypes.POINTER(Error))]
    lib.pp_revision_event_set_get.restype = ErrorCode
    lib.pp_revision_event_set_release.argtypes = [ctypes.POINTER(RevisionEventSet)]
    lib.pp_revision_event_set_release.restype = None
    lib.pp_production_resolve_asset.argtypes = [ctypes.POINTER(Production), ctypes.POINTER(Uuid), ctypes.POINTER(ctypes.POINTER(ResolutionSet)), ctypes.POINTER(ctypes.POINTER(Error))]
    lib.pp_production_resolve_asset.restype = ErrorCode
    lib.pp_resolution_set_representation_count.argtypes = [ctypes.POINTER(ResolutionSet)]
    lib.pp_resolution_set_representation_count.restype = ctypes.c_uint64
    lib.pp_resolution_set_get_representation.argtypes = [ctypes.POINTER(ResolutionSet), ctypes.c_uint64, ctypes.POINTER(Uuid), ctypes.POINTER(RepresentationAvailability), ctypes.POINTER(ctypes.c_uint64), ctypes.POINTER(ctypes.c_uint64), ctypes.POINTER(ctypes.POINTER(Error))]
    lib.pp_resolution_set_get_representation.restype = ErrorCode
    lib.pp_resolution_set_get_resource.argtypes = [ctypes.POINTER(ResolutionSet), ctypes.c_uint64, ctypes.c_uint64, ctypes.POINTER(Uuid), ctypes.POINTER(ResourceResolutionState), ctypes.POINTER(ctypes.c_uint64), ctypes.POINTER(ctypes.c_uint64), ctypes.POINTER(ctypes.POINTER(Error))]
    lib.pp_resolution_set_get_resource.restype = ErrorCode
    lib.pp_resolution_set_get_issue.argtypes = [ctypes.POINTER(ResolutionSet), ctypes.c_uint64, ctypes.c_uint64, ctypes.POINTER(Uuid), ctypes.POINTER(ctypes.c_uint8), ctypes.POINTER(AvailabilityIssueKind), ctypes.POINTER(ctypes.c_uint64), ctypes.POINTER(ctypes.POINTER(Error))]
    lib.pp_resolution_set_get_issue.restype = ErrorCode
    lib.pp_resolution_set_get_issue_frame.argtypes = [ctypes.POINTER(ResolutionSet), ctypes.c_uint64, ctypes.c_uint64, ctypes.c_uint64, ctypes.POINTER(ctypes.c_int64), ctypes.POINTER(ctypes.POINTER(Error))]
    lib.pp_resolution_set_get_issue_frame.restype = ErrorCode
    lib.pp_resolution_set_get_candidate.argtypes = [ctypes.POINTER(ResolutionSet), ctypes.c_uint64, ctypes.c_uint64, ctypes.c_uint64, ctypes.POINTER(ctypes.c_char_p), ctypes.POINTER(ctypes.c_uint16), ctypes.POINTER(ctypes.c_uint64), ctypes.POINTER(ctypes.POINTER(Error))]
    lib.pp_resolution_set_get_candidate.restype = ErrorCode
    lib.pp_resolution_set_get_resource_evidence.argtypes = [ctypes.POINTER(ResolutionSet), ctypes.c_uint64, ctypes.c_uint64, ctypes.c_uint64, ctypes.POINTER(EvidenceKind), ctypes.POINTER(ctypes.c_char_p), ctypes.POINTER(ctypes.POINTER(Error))]
    lib.pp_resolution_set_get_resource_evidence.restype = ErrorCode
    lib.pp_resolution_set_get_candidate_evidence.argtypes = [ctypes.POINTER(ResolutionSet), ctypes.c_uint64, ctypes.c_uint64, ctypes.c_uint64, ctypes.c_uint64, ctypes.POINTER(EvidenceKind), ctypes.POINTER(ctypes.c_char_p), ctypes.POINTER(ctypes.POINTER(Error))]
    lib.pp_resolution_set_get_candidate_evidence.restype = ErrorCode
    lib.pp_resolution_set_release.argtypes = [ctypes.POINTER(ResolutionSet)]
    lib.pp_resolution_set_release.restype = None
    lib.pp_production_begin_transaction.argtypes = [ctypes.POINTER(Production), ctypes.POINTER(ctypes.POINTER(Transaction)), ctypes.POINTER(ctypes.POINTER(Error))]
    lib.pp_production_begin_transaction.restype = ErrorCode
    lib.pp_production_release.argtypes = [ctypes.POINTER(Production)]
    lib.pp_production_release.restype = None
    lib.pp_transaction_set_revision_context.argtypes = [ctypes.POINTER(Transaction), ctypes.c_char_p, ctypes.c_char_p, ctypes.c_char_p, ctypes.c_char_p, ctypes.POINTER(ctypes.POINTER(Error))]
    lib.pp_transaction_set_revision_context.restype = ErrorCode
    lib.pp_transaction_import_media.argtypes = [ctypes.POINTER(Transaction), ctypes.c_char_p, ctypes.c_char_p, ctypes.POINTER(Uuid), ctypes.POINTER(ctypes.POINTER(Error))]
    lib.pp_transaction_import_media.restype = ErrorCode
    lib.pp_transaction_add_single_file_representation.argtypes = [ctypes.POINTER(Transaction), ctypes.POINTER(Uuid), RepresentationKind, ctypes.c_char_p, ctypes.POINTER(Uuid), ctypes.POINTER(ctypes.POINTER(Error))]
    lib.pp_transaction_add_single_file_representation.restype = ErrorCode
    lib.pp_transaction_add_image_sequence_representation.argtypes = [ctypes.POINTER(Transaction), ctypes.POINTER(Uuid), RepresentationKind, ctypes.c_char_p, ctypes.c_char_p, ctypes.c_char_p, ctypes.c_uint8, ctypes.c_int64, ctypes.c_int64, ctypes.c_uint32, ctypes.c_uint32, ctypes.c_uint32, ctypes.POINTER(ctypes.c_int64), ctypes.c_uint64, ctypes.POINTER(Uuid), ctypes.POINTER(ctypes.POINTER(Error))]
    lib.pp_transaction_add_image_sequence_representation.restype = ErrorCode
    lib.pp_transaction_add_media_root.argtypes = [ctypes.POINTER(Transaction), ctypes.c_char_p, ctypes.c_char_p, ctypes.c_int32, ctypes.POINTER(Uuid), ctypes.POINTER(ctypes.POINTER(Error))]
    lib.pp_transaction_add_media_root.restype = ErrorCode
    lib.pp_transaction_confirm_locator.argtypes = [ctypes.POINTER(Transaction), ctypes.POINTER(Uuid), ctypes.c_char_p, ctypes.POINTER(ctypes.POINTER(Error))]
    lib.pp_transaction_confirm_locator.restype = ErrorCode
    lib.pp_transaction_add_external_identifier.argtypes = [ctypes.POINTER(Transaction), ctypes.POINTER(ObjectRef), ctypes.c_char_p, ctypes.c_char_p, ctypes.c_char_p, ctypes.POINTER(ctypes.POINTER(Error))]
    lib.pp_transaction_add_external_identifier.restype = ErrorCode
    lib.pp_transaction_remove_external_identifier.argtypes = [ctypes.POINTER(Transaction), ctypes.POINTER(ObjectRef), ctypes.c_char_p, ctypes.c_char_p, ctypes.c_char_p, ctypes.POINTER(ctypes.POINTER(Error))]
    lib.pp_transaction_remove_external_identifier.restype = ErrorCode
    lib.pp_transaction_add_metadata_value.argtypes = [ctypes.POINTER(Transaction), ctypes.POINTER(ObjectRef), ctypes.c_char_p, ctypes.c_char_p, ctypes.POINTER(MetadataInput), ctypes.POINTER(ctypes.POINTER(Error))]
    lib.pp_transaction_add_metadata_value.restype = ErrorCode
    lib.pp_transaction_remove_metadata_property.argtypes = [ctypes.POINTER(Transaction), ctypes.POINTER(ObjectRef), ctypes.c_char_p, ctypes.c_char_p, ctypes.POINTER(ctypes.POINTER(Error))]
    lib.pp_transaction_remove_metadata_property.restype = ErrorCode
    lib.pp_transaction_create_activity.argtypes = [ctypes.POINTER(Transaction), ctypes.c_char_p, ctypes.POINTER(ActivityEdge), ctypes.c_uint64, ctypes.POINTER(ActivityEdge), ctypes.c_uint64, ctypes.POINTER(ctypes.c_int64), ctypes.POINTER(ctypes.c_int64), ctypes.c_char_p, ctypes.c_char_p, ctypes.c_char_p, ctypes.c_char_p, ctypes.c_char_p, ctypes.c_char_p, ctypes.c_char_p, ctypes.POINTER(Uuid), ctypes.POINTER(ctypes.POINTER(Error))]
    lib.pp_transaction_create_activity.restype = ErrorCode
    lib.pp_transaction_commit.argtypes = [ctypes.POINTER(Transaction), ctypes.POINTER(ctypes.POINTER(Error))]
    lib.pp_transaction_commit.restype = ErrorCode
    lib.pp_transaction_rollback.argtypes = [ctypes.POINTER(Transaction), ctypes.POINTER(ctypes.POINTER(Error))]
    lib.pp_transaction_rollback.restype = ErrorCode
    lib.pp_transaction_release.argtypes = [ctypes.POINTER(Transaction)]
    lib.pp_transaction_release.restype = None
    lib.pp_error_code.argtypes = [ctypes.POINTER(Error)]
    lib.pp_error_code.restype = ErrorCode
    lib.pp_error_message.argtypes = [ctypes.POINTER(Error)]
    lib.pp_error_message.restype = ctypes.c_char_p
    lib.pp_error_release.argtypes = [ctypes.POINTER(Error)]
    lib.pp_error_release.restype = None
