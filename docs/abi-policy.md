# ABI policy

ABI version 1 is pre-release and may change during the 0.x series, with every
change recorded in the changelog and ABI tests. `pp_abi_version()` reports the
implemented version. Exported symbol names are unversioned until the first stable
release, but removals or signature changes require an explicit ABI-version bump.

## Types and ownership

Projects and errors are opaque handles. A successful creation/open call transfers
one project ownership reference to the caller, which releases it exactly once with
`pp_project_release`. Failed calls optionally transfer an error object, released
exactly once with `pp_error_release`. Release functions accept null as a no-op;
releasing the same non-null pointer twice is invalid.

`pp_uuid_t` is the only public layout-bearing domain type and contains exactly 16
network-order UUID bytes. Numeric errors are fixed-width values defined in the C
header.

## Strings and errors

Input strings are borrowed, NUL-terminated UTF-8 and may not contain embedded NUL.
Optional strings use null. Error messages are borrowed NUL-terminated UTF-8 owned
by their error object and remain valid until that object is released. Stable error
codes are the contract; message wording is diagnostic and may evolve.

## Panics and threading

Every exported operation contains Rust unwinding with `catch_unwind`. Panics are
translated to `PP_ERROR_INTERNAL`; no panic may cross the C boundary. Project
handles are not currently safe for concurrent access. Callers must externally
serialize use and must not release a handle while another thread uses it.

## Header compatibility

The hand-reviewed C header is authoritative. Rust implementation types, SQLite
types, allocation APIs, and standard-library layouts never cross the ABI.

## C++ wrapper

`postproject.hpp` is a header-only C++17 wrapper over the authoritative C API.
It owns project handles with RAII, is move-only, and converts failed status codes
to `postproject::Error`. The exception retains the stable `ErrorCode` and copies
diagnostic text before releasing the C error object. No exception crosses the C
ABI. Inputs containing embedded NUL bytes are rejected before calling C.

The wrapper adds no domain behavior and exposes no C++ standard-library type
through exported library symbols. Its source compatibility follows the 0.x
pre-release policy independently of the C ABI version.
