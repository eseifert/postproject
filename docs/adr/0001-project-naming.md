# ADR 0001: Project naming

- Status: Accepted
- Date: 2026-09-19

## Decision

The ecosystem, prose name, and CMake project are **PostProject**. The repository
is `postproject-org/postproject`; Rust crates are `postproject-*`; installed headers are
under `postproject/`; the C ABI uses `pp_`; the C++ namespace is `postproject`;
and the CLI is `postproject`.

Unix native library files retain the conventional `libpostproject` prefix.
Windows uses `postproject.dll` where the toolchain permits. Historical records
may retain old URLs when changing them would falsify the record.

## Consequences

Package metadata and documentation use one product identity while native linker
names remain conventional. Renaming the hosted repository is an owner action if
authenticated tooling is unavailable.
