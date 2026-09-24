# ADR 0021: Fingerprint observations and activity snapshots

- Status: Accepted
- Date: 2026-09-24

## Context

Fingerprint rows were immutable after import. An activity edge named a stable
representation but did not record which content that representation identified
when the activity used or generated it. Consequently provenance could describe
lineage but could not answer whether an artifact still corresponded to its
inputs.

W3C PROV-DM defines an entity as a thing with fixed aspects and treats a changed
state as a distinct entity. Its specialization relation lets a time- or
context-specific entity retain a relationship to the more general thing. PROV-O
exposes that relation as `prov:specializationOf`. PostProject's stable
representation identity deliberately denotes the general production object,
so replacing it with a new representation ID on every content observation
would break locator, host-binding, and application identity semantics.

## Decision

A resource or representation fingerprint is its current observation in one
algorithm/version domain. A transaction may explicitly record another
observation. An identical value is a no-op; a changed value moves the previous
row to append-only history, records the revision sequence that superseded it,
and emits a semantic event identifying the object and fingerprint domain.
Verification remains read-only and never records an observation implicitly.

Resource observation does not duplicate the representation-fingerprint
algorithm in the storage adapter. It atomically marks every owning
representation as requiring recomputation. Recording the recomputed
representation observation clears that marker. Callers therefore cannot
mistake an old aggregate digest for current evidence, and the media crate
remains the sole implementation of ADR 0015's structure-aware algorithm.

When storage creates an activity it captures each edge inside the same SQLite
transaction. The snapshot contains the transaction's future revision sequence
and every current representation fingerprint domain. An empty set explicitly
means no fingerprint was available. A missing snapshot means only that the edge
predates schema 7; migration never fabricates historical observations.

Input and output snapshots are both retained. Inputs support dependency
staleness; outputs distinguish an upstream change from overwriting the generated
artifact itself. Snapshot data supplied on an in-memory activity specification
is ignored and replaced by storage state.

## Standards impact

An edge snapshot maps conceptually to a specialized PROV Entity describing the
representation at its use or generation revision. PostProject deliberately
stores the specialization inline—stable representation ID plus fingerprint and
revision—rather than minting another public Entity ID. This preserves the host
identity contract but is not a lossless PROV serialization. A future PROV
exporter must mint specialization entities for snapshots rather than exporting
the mutable representation directly as the used/generated entity.

The decision was checked against the W3C Recommendations *PROV-DM: The PROV
Data Model*, sections 5.1.1, 5.1.8, and 5.5.1, and *PROV-O: The PROV Ontology*,
`prov:specializationOf`, on 2026-09-24.

## Consequences

Staleness can compare facts captured at generation with current production
knowledge without accessing the filesystem. Schema 7 adds history, dirty
aggregate markers, edge snapshots, and two event kinds. Schema 6 productions
open through a forward migration; their existing activities have absent
snapshots and therefore evaluate as indeterminate rather than fresh.

History increases storage use only when content is explicitly re-observed.
Fingerprint algorithms remain open-world and their byte values remain opaque.
