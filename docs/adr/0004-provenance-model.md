# ADR 0004: Activity-based provenance

- Status: Proposed
- Date: 2026-09-19

## Decision

Production provenance is stored as activities with representation input and
output edges. Activity kinds and edge roles are extensible identifiers. An
activity may record tool and agent identity, while parameters use the metadata
model.

Derived lineage is a traversal query, not the sole stored fact. Generation
cycles are rejected, fan-in/fan-out and cross-asset processing are supported,
and malformed persisted cycles produce bounded errors.

## Consequences

The model maps strongly at a conceptual level to W3C PROV activities and
entities and accommodates MovieLabs OMC concepts without claiming compliance or
inferring revision/variant semantics.
