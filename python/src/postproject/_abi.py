"""ctypes layouts and signatures for the current public C ABI."""

from __future__ import annotations

import ctypes


class Uuid(ctypes.Structure):
    """Fixed-layout UUID-compatible identifier."""

    _fields_ = [("bytes", ctypes.c_uint8 * 16)]


class ObjectRef(ctypes.Structure):
    """Tagged reference used by metadata and identifier events."""

    _fields_ = [("kind", ctypes.c_uint32), ("id", Uuid)]


class RevisionEvent(ctypes.Structure):
    """Borrowed fixed-layout semantic revision event."""

    _fields_ = [
        ("kind", ctypes.c_uint32),
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


def configure_api(lib: ctypes.CDLL) -> None:
    """Declare the signatures used by the initial high-level wrapper."""

    error_out = ctypes.POINTER(ctypes.c_void_p)
    handle_out = ctypes.POINTER(ctypes.c_void_p)
    uuid_pointer = ctypes.POINTER(Uuid)

    _status(lib.pp_project_create, [ctypes.c_char_p, ctypes.c_char_p, handle_out, error_out])
    _status(lib.pp_project_open, [ctypes.c_char_p, handle_out, error_out])
    _status(lib.pp_project_id, [ctypes.c_void_p, uuid_pointer, error_out])
    lib.pp_project_release.argtypes = [ctypes.c_void_p]
    lib.pp_project_release.restype = None

    _status(lib.pp_project_begin_transaction, [ctypes.c_void_p, handle_out, error_out])
    _status(
        lib.pp_transaction_set_revision_context,
        [
            ctypes.c_void_p,
            ctypes.c_char_p,
            ctypes.c_char_p,
            ctypes.c_char_p,
            ctypes.c_char_p,
            error_out,
        ],
    )
    _status(
        lib.pp_transaction_import_media,
        [ctypes.c_void_p, ctypes.c_char_p, ctypes.c_char_p, uuid_pointer, error_out],
    )
    _status(lib.pp_transaction_commit, [ctypes.c_void_p, error_out])
    _status(lib.pp_transaction_rollback, [ctypes.c_void_p, error_out])
    lib.pp_transaction_release.argtypes = [ctypes.c_void_p]
    lib.pp_transaction_release.restype = None

    _status(lib.pp_project_latest_revision, [ctypes.c_void_p, handle_out, error_out])
    _status(
        lib.pp_project_changes_since,
        [ctypes.c_void_p, ctypes.c_uint64, ctypes.c_uint32, handle_out, error_out],
    )
    lib.pp_revision_set_count.argtypes = [ctypes.c_void_p]
    lib.pp_revision_set_count.restype = ctypes.c_uint64
    _status(
        lib.pp_revision_set_get,
        [
            ctypes.c_void_p,
            ctypes.c_uint64,
            uuid_pointer,
            ctypes.POINTER(ctypes.c_uint64),
            uuid_pointer,
            ctypes.POINTER(ctypes.c_int64),
            ctypes.POINTER(ctypes.c_char_p),
            ctypes.POINTER(ctypes.c_char_p),
            ctypes.POINTER(ctypes.c_char_p),
            ctypes.POINTER(ctypes.c_char_p),
            error_out,
        ],
    )
    lib.pp_revision_set_release.argtypes = [ctypes.c_void_p]
    lib.pp_revision_set_release.restype = None

    _status(
        lib.pp_project_revision_events,
        [ctypes.c_void_p, uuid_pointer, handle_out, error_out],
    )
    lib.pp_revision_event_set_count.argtypes = [ctypes.c_void_p]
    lib.pp_revision_event_set_count.restype = ctypes.c_uint64
    _status(
        lib.pp_revision_event_set_get,
        [
            ctypes.c_void_p,
            ctypes.c_uint64,
            ctypes.POINTER(RevisionEvent),
            error_out,
        ],
    )
    lib.pp_revision_event_set_release.argtypes = [ctypes.c_void_p]
    lib.pp_revision_event_set_release.restype = None


def _status(function: ctypes._CFuncPtr, arguments: list[object]) -> None:
    function.argtypes = arguments
    function.restype = ctypes.c_uint32

