"""Immutable Python values copied from the native ABI."""

from __future__ import annotations

from dataclasses import dataclass
from typing import TypeAlias
from uuid import UUID


@dataclass(frozen=True, slots=True)
class _TypedId:
    value: UUID

    def __str__(self) -> str:
        return str(self.value)


class ProductionId(_TypedId):
    """Stable identity of one PostProject production."""

    __slots__ = ()


class AssetId(_TypedId):
    """Stable identity of one logical asset."""

    __slots__ = ()


class RepresentationId(_TypedId):
    """Stable identity of one usable asset representation."""

    __slots__ = ()


class ResourceId(_TypedId):
    """Stable identity of one storage resource."""

    __slots__ = ()


class LocatorId(_TypedId):
    """Stable identity of one resource locator."""

    __slots__ = ()


class MediaRootId(_TypedId):
    """Stable identity of one configured media root."""

    __slots__ = ()


class ActivityId(_TypedId):
    """Stable identity of one provenance activity."""

    __slots__ = ()


class RevisionId(_TypedId):
    """Stable identity of one committed revision."""

    __slots__ = ()


class TransactionId(_TypedId):
    """Stable identity of the transaction that produced a revision."""

    __slots__ = ()


ObjectReference: TypeAlias = (
    ProductionId | AssetId | RepresentationId | ResourceId | ActivityId
)


@dataclass(frozen=True, slots=True)
class ExternalIdentifier:
    """Opaque external identity preserved exactly as supplied."""

    scheme: str
    value: str
    qualifier: str | None = None


@dataclass(frozen=True, slots=True)
class MetadataProperty:
    """Vocabulary-qualified metadata property identity."""

    vocabulary: str
    property: str


@dataclass(frozen=True, slots=True)
class OriginIdentity:
    """Integrating application or process identity, not an authenticated user."""

    name: str
    version: str | None = None
    uri: str | None = None


@dataclass(frozen=True, slots=True)
class RevisionContext:
    """Optional context applied to a transaction's future revision."""

    origin: OriginIdentity | None = None
    message: str | None = None


@dataclass(frozen=True, slots=True)
class Revision:
    """One committed production mutation transaction."""

    id: RevisionId
    sequence: int
    transaction_id: TransactionId
    committed_at_unix_micros: int
    origin: OriginIdentity | None
    message: str | None
