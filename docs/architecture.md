# Architecture

## Boundaries and dependency direction

`postproject-core` owns stable IDs, domain values, errors, transaction semantics,
and domain-oriented service contracts. It has no dependency on SQLite, C/C++, Qt,
or any editor. Every other component may depend on core; core never depends on an
adapter.

`postproject-storage-sqlite` owns project-file migrations and transactional
persistence. It implements the core `ProjectRead`, `ProjectStore`, and
`ProjectStoreTransaction` contracts, which describe domain operations rather
than generic row CRUD. A later backend can implement the same boundary without
exposing its connection or query model.

`postproject-media` owns filesystem candidate discovery, fingerprinting, and
resolution policy. Candidate discovery, cheap filtering, and expensive
verification remain separate so indexing can be introduced without changing the
domain result types.

`postproject-ffi` exposes a manually designed C ABI with opaque handles and panic
containment. The header-only C++ wrapper calls only that ABI. `postproject-cli`
depends on the domain, media, and SQLite crates and exercises those services
without reimplementing their behavior.

## Why a C ABI

Rust provides memory safety and expressive domain modeling internally, while a C
ABI gives downstream C, C++, Qt, Python, and GObject consumers a conventional,
toolchain-neutral integration boundary. Rust types, layouts, panics, and ownership
conventions must not cross it.

## Current direction and deferred concerns

The current iteration adds external identifiers, structured metadata,
activity-based provenance, rational time, and a semantic revision journal in
that domain-first order. Each capability reaches persistence before the public C
ABI and language wrappers are expanded around it.

Timelines, collaboration, networking, media decoding, job execution, full
standards adapters, and editor-specific models remain outside the architecture.
