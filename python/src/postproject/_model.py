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
class MetadataString:
    value: str


@dataclass(frozen=True, slots=True)
class MetadataLanguageString:
    value: str
    language: str


@dataclass(frozen=True, slots=True)
class MetadataI64:
    value: int


@dataclass(frozen=True, slots=True)
class MetadataU64:
    value: int


@dataclass(frozen=True, slots=True)
class MetadataDecimal:
    coefficient: int
    scale: int


@dataclass(frozen=True, slots=True)
class MetadataBool:
    value: bool


@dataclass(frozen=True, slots=True)
class MetadataTimestamp:
    unix_micros: int


@dataclass(frozen=True, slots=True)
class MetadataUri:
    value: str


@dataclass(frozen=True, slots=True)
class MetadataBytes:
    value: bytes


@dataclass(frozen=True, slots=True)
class MetadataRational:
    numerator: int
    denominator: int


@dataclass(frozen=True, slots=True)
class MetadataList:
    values: tuple[MetadataValue, ...]


@dataclass(frozen=True, slots=True)
class MetadataStructField:
    name: str
    value: MetadataValue


@dataclass(frozen=True, slots=True)
class MetadataStruct:
    fields: tuple[MetadataStructField, ...]


@dataclass(frozen=True, slots=True)
class MetadataReference:
    target: ObjectReference


MetadataValue: TypeAlias = (
    MetadataString
    | MetadataLanguageString
    | MetadataI64
    | MetadataU64
    | MetadataDecimal
    | MetadataBool
    | MetadataTimestamp
    | MetadataUri
    | MetadataBytes
    | MetadataRational
    | MetadataList
    | MetadataStruct
    | MetadataReference
)


@dataclass(frozen=True, slots=True)
class MetadataAssertion:
    target: ObjectReference
    property: MetadataProperty
    value: MetadataValue


@dataclass(frozen=True, slots=True)
class AssetImportedEvent:
    asset_id: AssetId


@dataclass(frozen=True, slots=True)
class RepresentationAddedEvent:
    asset_id: AssetId
    representation_id: RepresentationId


@dataclass(frozen=True, slots=True)
class ResourceAddedEvent:
    resource_id: ResourceId


@dataclass(frozen=True, slots=True)
class RepresentationResourceAddedEvent:
    representation_id: RepresentationId
    resource_id: ResourceId
    structural_position: int


@dataclass(frozen=True, slots=True)
class LocatorAddedEvent:
    resource_id: ResourceId
    locator_id: LocatorId


@dataclass(frozen=True, slots=True)
class MediaRootAddedEvent:
    media_root_id: MediaRootId


@dataclass(frozen=True, slots=True)
class ExternalIdentifierAddedEvent:
    target: ObjectReference
    identifier: ExternalIdentifier


@dataclass(frozen=True, slots=True)
class ExternalIdentifierRemovedEvent:
    target: ObjectReference
    identifier: ExternalIdentifier


@dataclass(frozen=True, slots=True)
class MetadataAddedOrReplacedEvent:
    target: ObjectReference
    property: MetadataProperty


@dataclass(frozen=True, slots=True)
class MetadataRemovedEvent:
    target: ObjectReference
    property: MetadataProperty


@dataclass(frozen=True, slots=True)
class ActivityCreatedEvent:
    activity_id: ActivityId
    kind: str


@dataclass(frozen=True, slots=True)
class ActivityInputAddedEvent:
    activity_id: ActivityId
    representation_id: RepresentationId
    role: str | None = None


@dataclass(frozen=True, slots=True)
class ActivityOutputAddedEvent:
    activity_id: ActivityId
    representation_id: RepresentationId
    role: str | None = None


RevisionEventPayload: TypeAlias = (
    AssetImportedEvent
    | RepresentationAddedEvent
    | ResourceAddedEvent
    | RepresentationResourceAddedEvent
    | LocatorAddedEvent
    | MediaRootAddedEvent
    | ExternalIdentifierAddedEvent
    | ExternalIdentifierRemovedEvent
    | MetadataAddedOrReplacedEvent
    | MetadataRemovedEvent
    | ActivityCreatedEvent
    | ActivityInputAddedEvent
    | ActivityOutputAddedEvent
)


@dataclass(frozen=True, slots=True)
class RevisionEvent:
    """One ordered semantic event within a revision."""

    position: int
    payload: RevisionEventPayload


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
