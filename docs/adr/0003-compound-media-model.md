# ADR 0003: Compound media model

- Status: Accepted
- Date: 2026-09-20

## Context

A useful media representation is not always one file. Image sequences, split
recordings, sidecars, and directory packages need identity, storage, and
availability semantics that do not expand one logical representation into an
unbounded set of objects.

Attaching paths, file facts, and content fingerprints directly to a
representation makes those cases ambiguous. It also confuses the identity of
stored bytes with the identity of the representation assembled from them.

## Decision

The durable ownership chain is:

`Asset -> Representation -> ContentStructure -> Resource -> Locator`

A representation identifies one usable realization of an asset. Its content
structure is one of these explicit shapes:

- a single resource;
- a compact image-sequence descriptor;
- ordered parts or spans; or
- a package or resource collection.

A resource identifies stored content and owns file facts and resource-level
fingerprints. A locator identifies one place or access route for a resource.
A resource may have several locators, and locator changes do not change content
identity.

Resource membership records an open-world role, ordering where the structure
requires it, and whether the member is required. Image sequences use a compact
pattern, frame range, and rational rate rather than one database row per frame.

A representation may additionally own a representation-level fingerprint over
its canonical structure. Resource and representation fingerprints are distinct
domains even when they use the same digest algorithm.

Resolution evaluates the complete content structure and reports one of
`Online`, `Partial`, `Offline`, or `Ambiguous`, with resource-level diagnostics.
It never silently treats one available member as a complete representation.

## Consequences

Single-file media remains the smallest content structure rather than a special
top-level model. Compound media can be persisted and resolved without losing
member roles or materializing every frame. Metadata and external identifiers
can target resources independently of representations.

The existing representation-to-location shape is replaced without a
compatibility layer.
