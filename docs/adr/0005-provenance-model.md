# ADR 0005: Activity-based provenance

- Status: Accepted
- Date: 2026-09-19

## Decision

Production provenance is stored as activities with representation input and
output edges. Activity kinds and edge roles are extensible identifiers. An
activity may record tool and agent identity, while parameters use the metadata
model.

Derived lineage is a traversal query, not the sole stored fact. Generation
cycles are rejected, fan-in/fan-out and cross-asset processing are supported,
and malformed persisted cycles produce bounded errors.

Activities are complete facts in the current model: they may have no inputs,
but must have at least one output because no in-progress lifecycle is exposed.
Identical edges and a representation appearing on both sides are rejected.
Timestamps, tool identity, agent identity, and edge roles are optional and
bounded. Metadata assertions carry activity parameters.

Schema version 7 adds storage-captured fingerprint and revision snapshots to
both input and output edges. The caller still specifies only representation and
role; persistence captures current state inside the activity transaction.
Pre-schema-7 edges retain an absent snapshot rather than receiving fabricated
history. See ADR 0021.

SQLite schema version 2 stores activities and their edges. Creation and cycle
validation occur inside the surrounding production transaction. Read APIs return
deterministically ordered activities and support producing/consuming lookup plus
transitive ancestor/descendant traversal.

## Consequences

The model maps strongly at a conceptual level to W3C PROV activities and
entities and accommodates MovieLabs OMC concepts without claiming compliance or
inferring revision/variant semantics.
