# ADR 0010: OpenAssetIO boundary

- Status: Accepted
- Date: 2026-09-20

## Context

Applications that need an interchangeable asset manager already have an
industry host-to-manager contract. A second PostProject-specific host protocol
would increase integration cost and couple applications to one implementation.
At the same time, PostProject has embedded and specialized capabilities that do
not belong in a general manager interface.

## Decision

OpenAssetIO is the preferred interoperability contract between a host
application and an asset manager. PostProject will be able to act as a manager
through a future `postproject-openassetio` adapter.

The adapter will translate:

- opaque entity references to stable PostProject objects;
- locatable content to representations, resources, and locators;
- image-collection traits to compact image-sequence structures;
- manager resolution requests to PostProject resolution results; and
- publishing operations to PostProject transactions, provenance, and revisions.

OpenAssetIO and OpenAssetIO-MediaCreation types do not enter
`postproject-core`. Their evolving traits and version-specific behavior remain
at the adapter boundary. A full manager implementation and publishing workflow
are deferred until the core compound-media model is proven.

PostProject retains its native API for embedded use and for specialized
capabilities such as fingerprint evidence, provenance traversal, and revision
feeds.

## Consequences

A host can support PostProject without hard-coding it as the only asset manager,
and PostProject can evolve its internal persistence independently of an external
contract. Mapping tests must demonstrate that one image sequence remains one
representation and that locatable content does not collapse resources into
representations.
