"""Owned production and transaction wrappers over the public C ABI."""

from __future__ import annotations

import ctypes
import os
import weakref
from _ctypes import _Pointer
from collections.abc import Callable
from pathlib import Path
from types import TracebackType
from typing import TypeVar
from uuid import UUID

from . import _abi
from ._abi import (
    Error,
    ExternalIdentifierSet,
    MetadataSet,
    MetadataValue as NativeMetadataValue,
    ObjectRefSet,
    Production as NativeProduction,
    RevisionEvent as NativeRevisionEvent,
    RevisionEventSet,
    RevisionSet,
    Transaction as NativeTransaction,
    Uuid,
)
from ._model import (
    ActivityCreatedEvent,
    ActivityId,
    ActivityInputAddedEvent,
    ActivityOutputAddedEvent,
    AssetImportedEvent,
    AssetId,
    ExternalIdentifier,
    ExternalIdentifierAddedEvent,
    ExternalIdentifierRemovedEvent,
    LocatorAddedEvent,
    LocatorId,
    MediaRootAddedEvent,
    MediaRootId,
    MetadataAddedOrReplacedEvent,
    MetadataAssertion,
    MetadataBool,
    MetadataBytes,
    MetadataDecimal,
    MetadataI64,
    MetadataLanguageString,
    MetadataList,
    MetadataProperty,
    MetadataRational,
    MetadataReference,
    MetadataRemovedEvent,
    MetadataString,
    MetadataStruct,
    MetadataStructField,
    MetadataTimestamp,
    MetadataU64,
    MetadataUri,
    MetadataValue,
    ObjectReference,
    OriginIdentity,
    ProductionId,
    RepresentationAddedEvent,
    RepresentationId,
    RepresentationResourceAddedEvent,
    ResourceAddedEvent,
    ResourceId,
    Revision,
    RevisionContext,
    RevisionEvent,
    RevisionId,
    TransactionId,
)
from ._native import NativeLibrary

_ProductionT = TypeVar("_ProductionT", bound="Production")


class _Assets:
    def __init__(self, production: Production) -> None:
        self._production = production

    def __contains__(self, asset_id: object) -> bool:
        if not isinstance(asset_id, AssetId):
            return False
        return self._production._contains_asset(asset_id)


class _ExternalIdentifiers:
    def __init__(self, production: Production) -> None:
        self._production = production

    def __getitem__(self, target: ObjectReference) -> tuple[ExternalIdentifier, ...]:
        return self._production._external_identifiers(target)


class _ObjectsByExternalIdentifier:
    def __init__(self, production: Production) -> None:
        self._production = production

    def __getitem__(self, key: tuple[str, str]) -> tuple[ObjectReference, ...]:
        scheme, value = key
        return self._production._find_by_external_identifier(scheme, value)


class _Metadata:
    def __init__(self, production: Production) -> None:
        self._production = production

    def __getitem__(self, target: ObjectReference) -> tuple[MetadataAssertion, ...]:
        return self._production._metadata(target)


class _MetadataByProperty:
    def __init__(self, production: Production) -> None:
        self._production = production

    def __getitem__(
        self, property: MetadataProperty
    ) -> tuple[MetadataAssertion, ...]:
        return self._production._metadata_by_property(property)


class _RevisionEvents:
    def __init__(self, production: Production) -> None:
        self._production = production

    def __getitem__(self, revision_id: RevisionId) -> tuple[RevisionEvent, ...]:
        return self._production._revision_events(revision_id)


class Production:
    """An owned native production handle."""

    def __init__(
        self,
        native: NativeLibrary,
        handle: _Pointer[NativeProduction],
    ) -> None:
        self._native = native
        self._handle = handle
        self._finalizer = weakref.finalize(
            self, native.lib.pp_production_release, handle
        )

    @classmethod
    def create(
        cls: type[_ProductionT],
        path: str | os.PathLike[str],
        display_name: str | None = None,
        *,
        library_path: str | os.PathLike[str] | None = None,
    ) -> _ProductionT:
        """Create a new production without overwriting an existing path."""

        native = NativeLibrary(library_path)
        handle = ctypes.POINTER(NativeProduction)()
        error = ctypes.POINTER(Error)()
        status = native.lib.pp_production_create(
            _path_bytes(path),
            _optional_text(display_name),
            ctypes.byref(handle),
            ctypes.byref(error),
        )
        native.check(status, error)
        if not handle:
            raise RuntimeError("native production creation returned no handle")
        return cls(native, handle)

    @classmethod
    def open(
        cls: type[_ProductionT],
        path: str | os.PathLike[str],
        *,
        library_path: str | os.PathLike[str] | None = None,
    ) -> _ProductionT:
        """Open an existing production."""

        native = NativeLibrary(library_path)
        handle = ctypes.POINTER(NativeProduction)()
        error = ctypes.POINTER(Error)()
        status = native.lib.pp_production_open(
            _path_bytes(path), ctypes.byref(handle), ctypes.byref(error)
        )
        native.check(status, error)
        if not handle:
            raise RuntimeError("native production open returned no handle")
        return cls(native, handle)

    @property
    def id(self) -> ProductionId:
        """Return this production's stable identity."""

        self._require_open()
        value = Uuid()
        error = ctypes.POINTER(Error)()
        status = self._native.lib.pp_production_id(
            self._handle, ctypes.byref(value), ctypes.byref(error)
        )
        self._native.check(status, error)
        return ProductionId(_uuid(value))

    @property
    def assets(self) -> _Assets:
        """Return an asset collection supporting ``asset_id in production.assets``."""

        self._require_open()
        return _Assets(self)

    @property
    def external_identifiers(self) -> _ExternalIdentifiers:
        """Return external identifiers keyed by their target object."""

        self._require_open()
        return _ExternalIdentifiers(self)

    @property
    def objects_by_external_identifier(self) -> _ObjectsByExternalIdentifier:
        """Return object matches keyed by ``(scheme, value)``."""

        self._require_open()
        return _ObjectsByExternalIdentifier(self)

    @property
    def metadata(self) -> _Metadata:
        """Return metadata assertions keyed by their target object."""

        self._require_open()
        return _Metadata(self)

    @property
    def metadata_by_property(self) -> _MetadataByProperty:
        """Return metadata assertions keyed by vocabulary-qualified property."""

        self._require_open()
        return _MetadataByProperty(self)

    @property
    def revision_events(self) -> _RevisionEvents:
        """Return semantic event lists keyed by revision identity."""

        self._require_open()
        return _RevisionEvents(self)

    def _contains_asset(self, asset_id: AssetId) -> bool:
        """Return whether an asset identity belongs to this production."""

        self._require_open()
        native_id = _native_uuid(asset_id.value)
        exists = ctypes.c_uint8()
        error = ctypes.POINTER(Error)()
        status = self._native.lib.pp_production_asset_exists(
            self._handle,
            ctypes.byref(native_id),
            ctypes.byref(exists),
            ctypes.byref(error),
        )
        self._native.check(status, error)
        return bool(exists.value)

    def _external_identifiers(
        self, target: ObjectReference
    ) -> tuple[ExternalIdentifier, ...]:
        """Return every external identifier attached to ``target``."""

        self._require_open()
        native_target = _native_object_reference(target)
        handle = ctypes.POINTER(ExternalIdentifierSet)()
        error = ctypes.POINTER(Error)()
        status = self._native.lib.pp_production_external_identifiers(
            self._handle,
            ctypes.byref(native_target),
            ctypes.byref(handle),
            ctypes.byref(error),
        )
        self._native.check(status, error)
        if not handle:
            raise RuntimeError("native identifier query returned no result set")
        try:
            count = self._native.lib.pp_external_identifier_set_count(handle)
            return tuple(
                _external_identifier_at(self._native, handle, index)
                for index in range(int(count))
            )
        finally:
            self._native.lib.pp_external_identifier_set_release(handle)

    def _find_by_external_identifier(
        self, scheme: str, value: str
    ) -> tuple[ObjectReference, ...]:
        """Find objects carrying an exact external scheme and value."""

        self._require_open()
        handle = ctypes.POINTER(ObjectRefSet)()
        error = ctypes.POINTER(Error)()
        status = self._native.lib.pp_production_find_by_external_identifier(
            self._handle,
            _utf8(scheme, "identifier scheme"),
            _utf8(value, "identifier value"),
            ctypes.byref(handle),
            ctypes.byref(error),
        )
        self._native.check(status, error)
        if not handle:
            raise RuntimeError("native identifier lookup returned no result set")
        try:
            count = self._native.lib.pp_object_ref_set_count(handle)
            return tuple(
                _object_reference_at(self._native, handle, index)
                for index in range(int(count))
            )
        finally:
            self._native.lib.pp_object_ref_set_release(handle)

    def _metadata(self, target: ObjectReference) -> tuple[MetadataAssertion, ...]:
        self._require_open()
        native_target = _native_object_reference(target)
        return self._metadata_set(
            self._native.lib.pp_production_metadata,
            self._handle,
            ctypes.byref(native_target),
        )

    def _metadata_by_property(
        self, property: MetadataProperty
    ) -> tuple[MetadataAssertion, ...]:
        self._require_open()
        return self._metadata_set(
            self._native.lib.pp_production_find_metadata,
            self._handle,
            _utf8(property.vocabulary, "metadata vocabulary"),
            _utf8(property.property, "metadata property"),
        )

    @property
    def latest_revision(self) -> Revision | None:
        """Return the newest committed revision, if one exists."""

        self._require_open()
        revisions = self._revision_set(
            self._native.lib.pp_production_latest_revision, self._handle
        )
        if len(revisions) > 1:
            raise RuntimeError("native latest-revision query returned multiple values")
        return revisions[0] if revisions else None

    def changes_since(self, sequence: int, limit: int) -> tuple[Revision, ...]:
        """Return a bounded ascending page of revisions after ``sequence``."""

        self._require_open()
        return self._revision_set(
            self._native.lib.pp_production_changes_since,
            self._handle,
            sequence,
            limit,
        )

    def _revision_events(self, revision_id: RevisionId) -> tuple[RevisionEvent, ...]:
        """Return the ordered semantic events for one revision."""

        self._require_open()
        native_id = _native_uuid(revision_id.value)
        handle = ctypes.POINTER(RevisionEventSet)()
        error = ctypes.POINTER(Error)()
        status = self._native.lib.pp_production_revision_events(
            self._handle,
            ctypes.byref(native_id),
            ctypes.byref(handle),
            ctypes.byref(error),
        )
        self._native.check(status, error)
        if not handle:
            raise RuntimeError("native revision-event query returned no result set")
        try:
            count = self._native.lib.pp_revision_event_set_count(handle)
            return tuple(
                _revision_event_at(self._native, handle, index)
                for index in range(int(count))
            )
        finally:
            self._native.lib.pp_revision_event_set_release(handle)

    def transaction(
        self,
        *,
        origin: OriginIdentity | str | None = None,
        message: str | None = None,
    ) -> Transaction:
        """Begin a transaction that commits on a clean context-manager exit."""

        self._require_open()
        handle = ctypes.POINTER(NativeTransaction)()
        error = ctypes.POINTER(Error)()
        status = self._native.lib.pp_production_begin_transaction(
            self._handle, ctypes.byref(handle), ctypes.byref(error)
        )
        self._native.check(status, error)
        if not handle:
            raise RuntimeError("native transaction creation returned no handle")
        transaction = Transaction(self._native, handle)
        if origin is not None or message is not None:
            identity = OriginIdentity(origin) if isinstance(origin, str) else origin
            transaction.set_revision_context(RevisionContext(identity, message))
        return transaction

    def close(self) -> None:
        """Release the native handle. Repeated calls are harmless."""

        self._finalizer()
        self._handle = ctypes.POINTER(NativeProduction)()

    def __enter__(self: _ProductionT) -> _ProductionT:
        self._require_open()
        return self

    def __exit__(
        self,
        exception_type: type[BaseException] | None,
        exception: BaseException | None,
        traceback: TracebackType | None,
    ) -> None:
        self.close()

    def _require_open(self) -> None:
        if not self._finalizer.alive:
            raise RuntimeError("production is closed")

    def _revision_set(
        self, function: Callable[..., int], *arguments: object
    ) -> tuple[Revision, ...]:
        handle = ctypes.POINTER(RevisionSet)()
        error = ctypes.POINTER(Error)()
        status = function(*arguments, ctypes.byref(handle), ctypes.byref(error))
        self._native.check(status, error)
        if not handle:
            raise RuntimeError("native revision query returned no result set")
        try:
            return tuple(
                _revision_at(self._native, handle, index)
                for index in range(int(self._native.lib.pp_revision_set_count(handle)))
            )
        finally:
            self._native.lib.pp_revision_set_release(handle)

    def _metadata_set(
        self, function: Callable[..., int], *arguments: object
    ) -> tuple[MetadataAssertion, ...]:
        handle = ctypes.POINTER(MetadataSet)()
        error = ctypes.POINTER(Error)()
        status = function(*arguments, ctypes.byref(handle), ctypes.byref(error))
        self._native.check(status, error)
        if not handle:
            raise RuntimeError("native metadata query returned no result set")
        try:
            count = self._native.lib.pp_metadata_set_count(handle)
            return tuple(
                _metadata_at(self._native, handle, index)
                for index in range(int(count))
            )
        finally:
            self._native.lib.pp_metadata_set_release(handle)


class Transaction:
    """An owned transaction with commit-or-rollback context semantics."""

    def __init__(
        self,
        native: NativeLibrary,
        handle: _Pointer[NativeTransaction],
    ) -> None:
        self._native = native
        self._handle = handle
        self._finished = False
        self._finalizer = weakref.finalize(
            self, native.lib.pp_transaction_release, handle
        )

    def set_revision_context(self, context: RevisionContext) -> None:
        """Set optional origin and message fields for the future revision."""

        self._require_open()
        origin = context.origin
        error = ctypes.POINTER(Error)()
        status = self._native.lib.pp_transaction_set_revision_context(
            self._handle,
            _optional_text(origin.name if origin else None),
            _optional_text(origin.version if origin else None),
            _optional_text(origin.uri if origin else None),
            _optional_text(context.message),
            ctypes.byref(error),
        )
        self._native.check(status, error)

    def import_media(
        self, path: str | os.PathLike[str], display_name: str | None = None
    ) -> AssetId:
        """Prepare and stage one original-media import."""

        self._require_open()
        asset_id = Uuid()
        error = ctypes.POINTER(Error)()
        status = self._native.lib.pp_transaction_import_media(
            self._handle,
            _path_bytes(path),
            _optional_text(display_name),
            ctypes.byref(asset_id),
            ctypes.byref(error),
        )
        self._native.check(status, error)
        return AssetId(_uuid(asset_id))

    def add_external_identifier(
        self, target: ObjectReference, identifier: ExternalIdentifier
    ) -> None:
        """Stage an external identifier attachment."""

        self._mutate_external_identifier(False, target, identifier)

    def remove_external_identifier(
        self, target: ObjectReference, identifier: ExternalIdentifier
    ) -> None:
        """Stage removal of one exact external identifier attachment."""

        self._mutate_external_identifier(True, target, identifier)

    def commit(self) -> None:
        """Atomically persist every staged mutation."""

        self._finish(self._native.lib.pp_transaction_commit)

    def rollback(self) -> None:
        """Discard every staged mutation."""

        self._finish(self._native.lib.pp_transaction_rollback)

    def close(self) -> None:
        """Release the handle, implicitly rolling back if still open."""

        self._finalizer()
        self._handle = ctypes.POINTER(NativeTransaction)()

    def __enter__(self) -> Transaction:
        self._require_open()
        return self

    def __exit__(
        self,
        exception_type: type[BaseException] | None,
        exception: BaseException | None,
        traceback: TracebackType | None,
    ) -> None:
        try:
            if not self._finished:
                if exception_type is None:
                    self.commit()
                else:
                    self.rollback()
        finally:
            self.close()

    def _finish(self, function: Callable[..., int]) -> None:
        self._require_open()
        error = ctypes.POINTER(Error)()
        status = function(self._handle, ctypes.byref(error))
        self._native.check(status, error)
        self._finished = True

    def _mutate_external_identifier(
        self,
        remove: bool,
        target: ObjectReference,
        identifier: ExternalIdentifier,
    ) -> None:
        self._require_open()
        native_target = _native_object_reference(target)
        error = ctypes.POINTER(Error)()
        function = (
            self._native.lib.pp_transaction_remove_external_identifier
            if remove
            else self._native.lib.pp_transaction_add_external_identifier
        )
        status = function(
            self._handle,
            ctypes.byref(native_target),
            _utf8(identifier.scheme, "identifier scheme"),
            _utf8(identifier.value, "identifier value"),
            None
            if identifier.qualifier is None
            else _utf8(identifier.qualifier, "identifier qualifier"),
            ctypes.byref(error),
        )
        self._native.check(status, error)

    def _require_open(self) -> None:
        if not self._finalizer.alive:
            raise RuntimeError("transaction is closed")
        if self._finished:
            raise RuntimeError("transaction is already finished")


def _path_bytes(path: str | os.PathLike[str]) -> bytes:
    return _utf8(str(Path(path)), "path")


def _optional_text(value: str | None) -> bytes | None:
    return None if value is None else _utf8(value, "text")


def _utf8(value: str, label: str) -> bytes:
    encoded = value.encode("utf-8")
    if b"\0" in encoded:
        raise ValueError(f"{label} must not contain NUL")
    return encoded


def _uuid(value: Uuid) -> UUID:
    return UUID(bytes=bytes(value.bytes))


def _native_uuid(value: UUID) -> Uuid:
    native = Uuid()
    native.bytes[:] = value.bytes
    return native


def _native_object_reference(value: ObjectReference) -> _abi.ObjectRef:
    native = _abi.ObjectRef()
    if isinstance(value, ProductionId):
        native.kind = _abi.PP_OBJECT_PRODUCTION
    elif isinstance(value, AssetId):
        native.kind = _abi.PP_OBJECT_ASSET
    elif isinstance(value, RepresentationId):
        native.kind = _abi.PP_OBJECT_REPRESENTATION
    elif isinstance(value, ResourceId):
        native.kind = _abi.PP_OBJECT_RESOURCE
    elif isinstance(value, ActivityId):
        native.kind = _abi.PP_OBJECT_ACTIVITY
    else:
        raise TypeError("unsupported object reference")
    native.id = _native_uuid(value.value)
    return native


def _external_identifier_at(
    native: NativeLibrary,
    identifiers: _Pointer[ExternalIdentifierSet],
    index: int,
) -> ExternalIdentifier:
    scheme = ctypes.c_char_p()
    value = ctypes.c_char_p()
    qualifier = ctypes.c_char_p()
    error = ctypes.POINTER(Error)()
    status = native.lib.pp_external_identifier_set_get(
        identifiers,
        index,
        ctypes.byref(scheme),
        ctypes.byref(value),
        ctypes.byref(qualifier),
        ctypes.byref(error),
    )
    native.check(status, error)
    return ExternalIdentifier(
        _decode_required(scheme.value, "identifier scheme"),
        _decode_required(value.value, "identifier value"),
        _decode_optional(qualifier.value),
    )


def _object_reference_at(
    native: NativeLibrary,
    objects: _Pointer[ObjectRefSet],
    index: int,
) -> ObjectReference:
    value = _abi.ObjectRef()
    error = ctypes.POINTER(Error)()
    status = native.lib.pp_object_ref_set_get(
        objects, index, ctypes.byref(value), ctypes.byref(error)
    )
    native.check(status, error)
    return _object_reference(value)


def _metadata_at(
    native: NativeLibrary,
    metadata: _Pointer[MetadataSet],
    index: int,
) -> MetadataAssertion:
    target = _abi.ObjectRef()
    vocabulary = ctypes.c_char_p()
    property_name = ctypes.c_char_p()
    value = ctypes.POINTER(NativeMetadataValue)()
    error = ctypes.POINTER(Error)()
    status = native.lib.pp_metadata_set_get(
        metadata,
        index,
        ctypes.byref(target),
        ctypes.byref(vocabulary),
        ctypes.byref(property_name),
        ctypes.byref(value),
        ctypes.byref(error),
    )
    native.check(status, error)
    if not value:
        raise RuntimeError("native metadata assertion is missing its value")
    return MetadataAssertion(
        _object_reference(target),
        MetadataProperty(
            _decode_required(vocabulary.value, "metadata vocabulary"),
            _decode_required(property_name.value, "metadata property"),
        ),
        _metadata_value(native, value),
    )


def _metadata_value(
    native: NativeLibrary, value: _Pointer[NativeMetadataValue]
) -> MetadataValue:
    kind = int(native.lib.pp_metadata_value_kind(value))
    error = ctypes.POINTER(Error)()

    if kind in (_abi.PP_METADATA_STRING, _abi.PP_METADATA_LANG_STRING):
        text = ctypes.c_char_p()
        language = ctypes.c_char_p()
        status = native.lib.pp_metadata_value_get_string(
            value, ctypes.byref(text), ctypes.byref(language), ctypes.byref(error)
        )
        native.check(status, error)
        decoded = _decode_required(text.value, "metadata text")
        if kind == _abi.PP_METADATA_STRING:
            return MetadataString(decoded)
        return MetadataLanguageString(
            decoded, _decode_required(language.value, "metadata language")
        )

    if kind == _abi.PP_METADATA_I64:
        result = ctypes.c_int64()
        status = native.lib.pp_metadata_value_get_i64(
            value, ctypes.byref(result), ctypes.byref(error)
        )
        native.check(status, error)
        return MetadataI64(int(result.value))

    if kind == _abi.PP_METADATA_U64:
        result = ctypes.c_uint64()
        status = native.lib.pp_metadata_value_get_u64(
            value, ctypes.byref(result), ctypes.byref(error)
        )
        native.check(status, error)
        return MetadataU64(int(result.value))

    if kind == _abi.PP_METADATA_DECIMAL:
        coefficient = ctypes.c_char_p()
        scale = ctypes.c_uint32()
        status = native.lib.pp_metadata_value_get_decimal(
            value,
            ctypes.byref(coefficient),
            ctypes.byref(scale),
            ctypes.byref(error),
        )
        native.check(status, error)
        coefficient_text = _decode_required(
            coefficient.value, "metadata decimal coefficient"
        )
        try:
            parsed_coefficient = int(coefficient_text)
        except ValueError as exception:
            raise RuntimeError("native metadata decimal is invalid") from exception
        return MetadataDecimal(parsed_coefficient, int(scale.value))

    if kind == _abi.PP_METADATA_BOOL:
        result = ctypes.c_uint8()
        status = native.lib.pp_metadata_value_get_bool(
            value, ctypes.byref(result), ctypes.byref(error)
        )
        native.check(status, error)
        return MetadataBool(bool(result.value))

    if kind == _abi.PP_METADATA_TIMESTAMP:
        result = ctypes.c_int64()
        status = native.lib.pp_metadata_value_get_timestamp(
            value, ctypes.byref(result), ctypes.byref(error)
        )
        native.check(status, error)
        return MetadataTimestamp(int(result.value))

    if kind == _abi.PP_METADATA_URI:
        result = ctypes.c_char_p()
        status = native.lib.pp_metadata_value_get_uri(
            value, ctypes.byref(result), ctypes.byref(error)
        )
        native.check(status, error)
        return MetadataUri(_decode_required(result.value, "metadata URI"))

    if kind == _abi.PP_METADATA_BYTES:
        result = ctypes.POINTER(ctypes.c_uint8)()
        length = ctypes.c_uint64()
        status = native.lib.pp_metadata_value_get_bytes(
            value,
            ctypes.byref(result),
            ctypes.byref(length),
            ctypes.byref(error),
        )
        native.check(status, error)
        return MetadataBytes(bytes(result[: length.value]) if length.value else b"")

    if kind == _abi.PP_METADATA_RATIONAL:
        numerator = ctypes.c_int64()
        denominator = ctypes.c_uint64()
        status = native.lib.pp_metadata_value_get_rational(
            value,
            ctypes.byref(numerator),
            ctypes.byref(denominator),
            ctypes.byref(error),
        )
        native.check(status, error)
        return MetadataRational(int(numerator.value), int(denominator.value))

    if kind == _abi.PP_METADATA_LIST:
        count = native.lib.pp_metadata_value_list_count(value)
        items: list[MetadataValue] = []
        for index in range(int(count)):
            item = ctypes.POINTER(NativeMetadataValue)()
            status = native.lib.pp_metadata_value_list_get(
                value, index, ctypes.byref(item), ctypes.byref(error)
            )
            native.check(status, error)
            if not item:
                raise RuntimeError("native metadata list item is missing")
            items.append(_metadata_value(native, item))
        return MetadataList(tuple(items))

    if kind == _abi.PP_METADATA_STRUCT:
        count = native.lib.pp_metadata_value_struct_count(value)
        fields: list[MetadataStructField] = []
        for index in range(int(count)):
            name = ctypes.c_char_p()
            field_value = ctypes.POINTER(NativeMetadataValue)()
            status = native.lib.pp_metadata_value_struct_get(
                value,
                index,
                ctypes.byref(name),
                ctypes.byref(field_value),
                ctypes.byref(error),
            )
            native.check(status, error)
            if not field_value:
                raise RuntimeError("native metadata structure field is missing")
            fields.append(
                MetadataStructField(
                    _decode_required(name.value, "metadata field name"),
                    _metadata_value(native, field_value),
                )
            )
        return MetadataStruct(tuple(fields))

    if kind == _abi.PP_METADATA_REFERENCE:
        target = _abi.ObjectRef()
        status = native.lib.pp_metadata_value_get_reference(
            value, ctypes.byref(target), ctypes.byref(error)
        )
        native.check(status, error)
        return MetadataReference(_object_reference(target))

    raise RuntimeError("metadata value has an unknown semantic kind")


def _revision_at(
    native: NativeLibrary,
    revisions: _Pointer[RevisionSet],
    index: int,
) -> Revision:
    revision_id = Uuid()
    sequence = ctypes.c_uint64()
    transaction_id = Uuid()
    committed_at = ctypes.c_int64()
    origin_name = ctypes.c_char_p()
    origin_version = ctypes.c_char_p()
    origin_uri = ctypes.c_char_p()
    message = ctypes.c_char_p()
    error = ctypes.POINTER(Error)()
    status = native.lib.pp_revision_set_get(
        revisions,
        index,
        ctypes.byref(revision_id),
        ctypes.byref(sequence),
        ctypes.byref(transaction_id),
        ctypes.byref(committed_at),
        ctypes.byref(origin_name),
        ctypes.byref(origin_version),
        ctypes.byref(origin_uri),
        ctypes.byref(message),
        ctypes.byref(error),
    )
    native.check(status, error)
    name = _decode_optional(origin_name.value)
    origin = None
    if name is not None:
        origin = OriginIdentity(
            name,
            _decode_optional(origin_version.value),
            _decode_optional(origin_uri.value),
        )
    return Revision(
        RevisionId(_uuid(revision_id)),
        int(sequence.value),
        TransactionId(_uuid(transaction_id)),
        int(committed_at.value),
        origin,
        _decode_optional(message.value),
    )


def _revision_event_at(
    native: NativeLibrary,
    events: _Pointer[RevisionEventSet],
    index: int,
) -> RevisionEvent:
    event = NativeRevisionEvent()
    error = ctypes.POINTER(Error)()
    status = native.lib.pp_revision_event_set_get(
        events, index, ctypes.byref(event), ctypes.byref(error)
    )
    native.check(status, error)

    kind = int(event.kind)
    if kind == _abi.PP_REVISION_ASSET_IMPORTED:
        payload = AssetImportedEvent(AssetId(_uuid(event.asset_id)))
    elif kind == _abi.PP_REVISION_REPRESENTATION_ADDED:
        payload = RepresentationAddedEvent(
            AssetId(_uuid(event.asset_id)),
            RepresentationId(_uuid(event.representation_id)),
        )
    elif kind == _abi.PP_REVISION_RESOURCE_ADDED:
        payload = ResourceAddedEvent(ResourceId(_uuid(event.resource_id)))
    elif kind == _abi.PP_REVISION_REPRESENTATION_RESOURCE_ADDED:
        payload = RepresentationResourceAddedEvent(
            RepresentationId(_uuid(event.representation_id)),
            ResourceId(_uuid(event.resource_id)),
            int(event.structural_position),
        )
    elif kind == _abi.PP_REVISION_LOCATOR_ADDED:
        payload = LocatorAddedEvent(
            ResourceId(_uuid(event.resource_id)),
            LocatorId(_uuid(event.locator_id)),
        )
    elif kind == _abi.PP_REVISION_MEDIA_ROOT_ADDED:
        payload = MediaRootAddedEvent(MediaRootId(_uuid(event.media_root_id)))
    elif kind == _abi.PP_REVISION_EXTERNAL_IDENTIFIER_ADDED:
        payload = ExternalIdentifierAddedEvent(
            _object_reference(event.target), _external_identifier(event)
        )
    elif kind == _abi.PP_REVISION_EXTERNAL_IDENTIFIER_REMOVED:
        payload = ExternalIdentifierRemovedEvent(
            _object_reference(event.target), _external_identifier(event)
        )
    elif kind == _abi.PP_REVISION_METADATA_ADDED_OR_REPLACED:
        payload = MetadataAddedOrReplacedEvent(
            _object_reference(event.target), _metadata_property(event)
        )
    elif kind == _abi.PP_REVISION_METADATA_REMOVED:
        payload = MetadataRemovedEvent(
            _object_reference(event.target), _metadata_property(event)
        )
    elif kind == _abi.PP_REVISION_ACTIVITY_CREATED:
        payload = ActivityCreatedEvent(
            ActivityId(_uuid(event.activity_id)),
            _decode_required(event.activity_kind, "activity kind"),
        )
    elif kind == _abi.PP_REVISION_ACTIVITY_INPUT_ADDED:
        payload = ActivityInputAddedEvent(
            ActivityId(_uuid(event.activity_id)),
            RepresentationId(_uuid(event.representation_id)),
            _decode_optional(event.role),
        )
    elif kind == _abi.PP_REVISION_ACTIVITY_OUTPUT_ADDED:
        payload = ActivityOutputAddedEvent(
            ActivityId(_uuid(event.activity_id)),
            RepresentationId(_uuid(event.representation_id)),
            _decode_optional(event.role),
        )
    else:
        raise RuntimeError("revision event has an unknown semantic kind")

    return RevisionEvent(int(event.position), payload)


def _external_identifier(event: NativeRevisionEvent) -> ExternalIdentifier:
    return ExternalIdentifier(
        _decode_required(event.identifier_scheme, "identifier scheme"),
        _decode_required(event.identifier_value, "identifier value"),
        _decode_optional(event.identifier_qualifier),
    )


def _metadata_property(event: NativeRevisionEvent) -> MetadataProperty:
    return MetadataProperty(
        _decode_required(event.vocabulary, "metadata vocabulary"),
        _decode_required(event.property, "metadata property"),
    )


def _object_reference(value: _abi.ObjectRef) -> ObjectReference:
    object_id = _uuid(value.id)
    kind = int(value.kind)
    if kind == _abi.PP_OBJECT_PRODUCTION:
        return ProductionId(object_id)
    if kind == _abi.PP_OBJECT_ASSET:
        return AssetId(object_id)
    if kind == _abi.PP_OBJECT_REPRESENTATION:
        return RepresentationId(object_id)
    if kind == _abi.PP_OBJECT_RESOURCE:
        return ResourceId(object_id)
    if kind == _abi.PP_OBJECT_ACTIVITY:
        return ActivityId(object_id)
    raise RuntimeError("revision event has an unknown object-reference kind")


def _decode_required(value: bytes | None, label: str) -> str:
    if value is None:
        raise RuntimeError(f"revision event is missing {label}")
    return value.decode("utf-8")


def _decode_optional(value: bytes | None) -> str | None:
    return None if value is None else value.decode("utf-8")
