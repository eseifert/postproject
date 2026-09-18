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
