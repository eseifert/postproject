# ABI policy

ABI version 14 is pre-release and may change during the 0.x series, with every
change recorded in the changelog and ABI tests. `pp_abi_version()` reports the
implemented version. Exported symbol names are unversioned until the first stable
release, but removals or signature changes require an explicit ABI-version bump.

## Types and ownership

Productions, transactions, asset sets, media-root sets, representation sets,
resolution sets, activity sets, external-identifier sets, object-reference sets,
metadata inputs, and errors are opaque handles. A
successful creation/open call transfers one production ownership reference to the
caller, which releases it exactly once with `pp_production_release`. Failed calls
optionally transfer an error object, released exactly once with
`pp_error_release`. Release functions accept null as a no-op; releasing the same
non-null pointer twice is invalid.

A production permits one open transaction at a time. Import and media-root mutations
are prepared and staged in memory, then persisted together by
`pp_transaction_commit`. Rollback or release of an open transaction discards all
staged work. A transaction retains the underlying production state, so its handle
remains valid if the originating production handle is released. Closed transaction
handles may only be released.

`pp_uuid_t` contains exactly 16 network-order UUID bytes. `pp_object_ref_t`
combines that ID with a fixed-width object-kind tag; open-world concepts such as
identifier schemes remain UTF-8 strings rather than C enums. Numeric errors and
object kinds are fixed-width values defined in the C header.

## Strings and errors

Input strings are borrowed, NUL-terminated UTF-8 and may not contain embedded NUL.
Optional strings use null. Error messages are borrowed NUL-terminated UTF-8 owned
by their error object and remain valid until that object is released. Stable error
codes are the contract; message wording is diagnostic and may evolve.

## Panics and threading

Every exported operation contains Rust unwinding with `catch_unwind`. Panics are
translated to `PP_ERROR_INTERNAL`; no panic may cross the C boundary. Production
handles may move between threads and support concurrent calls. Calls on one handle
serialize internally and block rather than reporting a contention conflict. A
panic while the handle is locked does not poison later calls.

Transaction, result-set, and error handles require caller-side serialization. No
handle may be released while another thread uses it. Transactions stage mutations
without holding the production lock; commit serializes with operations using the
same production state. Opening the production again provides a separate handle for
reads during that interval, subject to SQLite's own file-locking behavior.

## Header compatibility

The hand-reviewed C header is authoritative. Rust implementation types, SQLite
types, allocation APIs, and standard-library layouts never cross the ABI.

## C++ wrapper

`postproject.hpp` is a header-only C++17 wrapper over the authoritative C API.
It owns production and transaction handles with RAII, makes both wrappers move-only,
and converts failed status codes to `postproject::Error`. Destruction of an open
transaction invokes the C release behavior and therefore discards staged work.
The exception retains the stable `ErrorCode` and copies diagnostic text before
releasing the C error object. No exception crosses the C ABI. Inputs containing
embedded NUL bytes are rejected before calling C.

## Resolution results

`pp_production_resolve_asset` returns an immutable opaque set containing one result
per representation. Each representation reports aggregate availability,
ordered resource results, and availability issues such as offline required
resources or missing sequence frames. Fixed-width states, issue kinds, frames,
candidates, and evidence are read through index-checked accessors. Candidate URI
and optional evidence-detail strings are borrowed from the result set and remain
valid until `pp_resolution_set_release`. The C++ wrapper copies these into
`RepresentationResolution`, `ResourceResolution`, `AvailabilityIssue`,
`ResolutionCandidate`, and `Evidence` values, so their lifetime is independent
of the C handle.

Resolution never mutates a production. A caller explicitly stages a selected
candidate using `pp_transaction_confirm_locator`, and only transaction commit
makes that location durable. The caller is responsible for passing a URI from
the result it reviewed; the API validates the URI and resource identity at
persistence time but does not silently choose a candidate.

Resolution snapshots the database state it needs while holding the production
lock, then releases that lock before filesystem discovery and fingerprinting.

## Representation inspection

`pp_production_representations` returns immutable snapshots of an asset's
representations. Index-checked accessors expose structure kind, ordered members,
requiredness and roles, compact image-sequence descriptors, concrete resources,
locators, and their last observed availability. Resource fingerprints and
structure-aware representation fingerprints have separate accessors and counts;
callers must not treat one as the other. Returned strings and fingerprint byte
spans borrow the result set and remain valid until
`pp_representation_set_release`.

The C++ wrapper copies the complete snapshot into `Representation`, `Resource`,
`Locator`, and `Fingerprint` values. Compact image sequences remain one resource
with a pattern and frame domain rather than one synthetic resource per frame.

## External identifiers

External identifiers are staged with a typed object reference, scheme, opaque
value, and optional qualifier. Add/remove operations are atomic with every other
transaction mutation. Enumeration returns an owned result-set handle whose
strings remain borrowed until release. Exact scheme/value lookup returns a
separate owned object-reference set and does not normalize inputs or contact a
registry.

The wrapper adds no domain behavior and exposes no C++ standard-library type
through exported library symbols. Its source compatibility follows the 0.x
pre-release policy independently of the C ABI version.
