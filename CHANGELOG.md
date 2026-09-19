# Changelog

All notable changes to libpostproject will be documented here. The project uses
[Semantic Versioning](https://semver.org/) once a stable API is released.

## Unreleased

### Added

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
