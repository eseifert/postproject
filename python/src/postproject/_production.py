"""Owned production and transaction wrappers over the public C ABI."""

from __future__ import annotations

import ctypes
import os
import weakref
from pathlib import Path
from types import TracebackType
from typing import Self
from uuid import UUID

from ._abi import (
    Error,
    Production as NativeProduction,
    Transaction as NativeTransaction,
    Uuid,
)
from ._model import AssetId, OriginIdentity, ProductionId, RevisionContext
from ._native import NativeLibrary


class Production:
    """An owned native production handle."""

    def __init__(
        self,
        native: NativeLibrary,
        handle: ctypes.POINTER(NativeProduction),
    ) -> None:
        self._native = native
        self._handle = handle
        self._finalizer = weakref.finalize(
            self, native.lib.pp_production_release, handle
        )

    @classmethod
    def create(
        cls,
        path: str | os.PathLike[str],
        display_name: str | None = None,
        *,
        library_path: str | os.PathLike[str] | None = None,
    ) -> Self:
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
        cls,
        path: str | os.PathLike[str],
        *,
        library_path: str | os.PathLike[str] | None = None,
    ) -> Self:
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

    def contains_asset(self, asset_id: AssetId) -> bool:
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

    def __enter__(self) -> Self:
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


class Transaction:
    """An owned transaction with commit-or-rollback context semantics."""

    def __init__(
        self,
        native: NativeLibrary,
        handle: ctypes.POINTER(NativeTransaction),
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

    def __enter__(self) -> Self:
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

    def _finish(self, function: ctypes._CFuncPtr) -> None:
        self._require_open()
        error = ctypes.POINTER(Error)()
        status = function(self._handle, ctypes.byref(error))
        self._native.check(status, error)
        self._finished = True

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
