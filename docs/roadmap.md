# Roadmap

## Iteration 1 — complete

Deliver stable media identity, SQLite persistence, versioned fingerprints,
deterministic relinking, a C ABI, C++ wrapper, CLI, packaging, and comprehensive
tests. The delivered scope and remaining constraints are recorded in the
[acceptance report](iteration-one-report.md).

## Iteration 2 — complete

Establish standards-aware internal/external identity, structured and repeated
metadata, activity-based provenance, rational time, a durable semantic revision
journal, and a Python binding over the public C ABI. Documentation is organized
for application users, integrators, and contributors. The delivered scope and
remaining constraints are recorded in the
[acceptance report](iteration-two-report.md).

## Explicitly later

- the published documentation site at `postproject.org`;
- automatic compound-media recognition and technical inspection;
- a PostProject OpenAssetIO Manager and OTIO-through-OpenAssetIO demonstration;
- portable named-root or volume mappings across machines;
- full IPTC VMH and EBUCorePlus mapping packages;
- W3C PROV and MovieLabs OMC import/export adapters;
- C2PA assertion, signing, and verification integration;
- MXF, AAF, and IMF adapters;
- OTIO adapters beyond a possible media-linker proof;
- collections, asset groups, and package models;
- persistent filesystem indexing and background job execution;
- PostgreSQL, a network daemon, collaboration, locking, and conflict handling;
- timeline/editorial models, GObject/Qt adapters, extraction, transcription, and
  semantic search;
- remote object storage, undo/redo, and distributed revision merging.
