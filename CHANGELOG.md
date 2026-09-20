# Changelog

All notable changes to PostProject will be documented here. The project uses
[Semantic Versioning](https://semver.org/) once a stable API is released.

## Unreleased

### Changed

- Renamed the project and repository identity to PostProject and
  `eseifert/postproject`; native Unix library files retain the conventional
  `libpostproject` name.
- Advanced the development version to `0.2.0-alpha.1` for the standards-aware
  domain-model work. Pre-1.0 API, ABI, schema, CLI, and binding compatibility is
  not promised.
- Replaced the flat native media-resolution result with ABI version 5's nested
  representation availability, resource results, and availability issues.

### Added

- An mdBook documentation foundation split across application-user,
  integrator, contributor, concept, and reference sections.
- Architecture decisions for naming, identity, metadata, provenance, rational
  time, revision events, Python binding strategy, and explicit SQL persistence.
- Strong activity and revision IDs, typed cross-object references, and bounded
  extensible external-identifier domain values with exact opaque round-trips.
- Canonical SQLite schema version 2 with transactional asset/representation
  external identifiers, exact scheme/value lookup, rollback behavior, and clean
  rejection of the obsolete development schema.
- C ABI version 2 and C++17 wrappers for typed object references, external
  identifier enumeration, transactional add/remove, and exact lookup.
- Activity-based provenance with extensible kinds and edge roles, bounded tool
  and agent identity, atomic SQLite schema version 2 persistence, cycle
  prevention, deterministic reload, and ancestry/descendant queries.
- CLI commands for attaching, removing, listing, and finding external
  identifiers with structured JSON output.
- Initial workspace and engineering-policy scaffolding.
- Dual MIT or Apache-2.0 licensing.
- Strong project, asset, representation, location, media-root, and transaction IDs.
- Validated domain values for projects, media identity, fingerprints, and locations.
- Structured, deterministically ordered resolver results and evidence.
- Backend-neutral transaction lifecycle semantics.
- SQLite schema version 1 with transactional numbered migrations.
- SQLite-backed project creation/opening with durable stable identity.
- Automated advisory, source, duplicate-dependency, and license policy checks.
- Versioned full and sampled BLAKE3 fingerprints with explicit coverage evidence.
- Atomic original-media imports and media-root persistence through explicit transactions.
- Deterministic bounded media resolution with explicit ambiguity and confirmed relinks.
- C ABI version 1 foundation with opaque project/error handles and panic containment.
- Header-only C++17 RAII wrapper with typed exceptions and move-only project ownership.
- Installable CMake and `pkg-config` metadata with a standalone native consumer test.
- CLI project, media, root, inspection, resolution, and explicit confirmation workflows.
- Machine-readable CLI JSON output and an end-to-end ambiguous relinking test.
- Additive C ABI transaction, media-import, media-root, and asset-identity operations.
- C++17 RAII transaction wrapper for atomic imports, media roots, and rollback.
- Criterion baselines for bulk import, large-project open, resolver scans, and commits.
- Schema-0 migration fixture and full multi-asset relocation/ambiguity E2E coverage.
- `cargo-fuzz` targets for project files, fingerprints, C strings, and strong IDs.
- ASan/UBSan native-consumer CI and a standalone installed-package C example.
- Shared/static native artifacts, installed release documentation, and release checklist.
- Bounded SQLite value and row sizes when opening untrusted project files.
- Structured C and C++ media-resolution results with transactional confirmation.
- Unambiguous cross-platform CMake metadata with explicit Windows DLL packaging.
- Backend-neutral, domain-shaped project read and transaction contracts.
- Iteration-one acceptance report with verification and benchmark summaries.
- Locked the compatible `yoke-derive` patch release to preserve Rust 1.85 support.
