# ADR 0026: Domain query cursors

- Status: Accepted
- Date: 2026-09-25

## Context

Production-sized enumerations cannot safely return every matching row. The
existing direct-dependent and job reads do exactly that, while ADR 0024 defers
paginated direct and transitive dependency queries to the common query cursor.
Offset pagination would become progressively more expensive and can repeat or
skip rows when concurrent commits move offsets.

The cursor contract must work through Rust, C, C++, Python, and the CLI without
exposing a backend query language or SQLite row identities. Dependency graphs
may contain cycles, so a page-size limit alone does not bound traversal.

## Decision

Named domain queries use a common request and result shape:

- a page size from 1 through 1,000;
- an optional opaque continuation token returned by the preceding page;
- results in a documented stable key order; and
- an optional next token only when another page exists.

Tokens are versioned, query-scoped, and bound to every parameter that affects
membership or ordering. Reusing a token with another query, root object,
filter, or traversal bound is an invalid argument. Tokens are bounded UTF-8
values that callers may copy or serialize, but their contents are not an API
and may change before 1.0. They are continuations, not production identities,
revision cursors, authentication credentials, or durable snapshots.

SQLite adapters implement tokens as keyset positions. Queries compare the
stable domain key after the token and fetch one extra row to decide whether to
return a continuation. They never use `OFFSET` and never expose a SQLite row
number.

Dependency and dependent queries additionally require a maximum depth from 1
through 64 and a maximum of 1 through 1,000 traversed representations. Their
page reports explicit traversal truncation independently of the continuation
token. Cycles are suppressed within the bounded traversal. A depth of one is a
direct query; larger depths include the transitive closure. Forward results
name asset or representation targets and reverse results name dependent
representations, each with its shortest observed depth.

Job enumeration adopts the same cursor immediately because it is already an
unbounded whole-set read. It accepts optional exact state and kind filters and
orders by job ID. The existing job state/kind index backs the combined filter;
additional indexes are added only where an accepted query shape needs them.

Pages are weakly consistent across commits. A later page sees current durable
state after its key; callers that need change tracking use the semantic
revision feed. A concurrent insertion before an already-consumed key is not
retroactively included.

## Alternatives considered

Offsets were rejected because their cost grows with page position and their
meaning shifts under concurrent writes. Returning complete vectors with only a
maximum-result error was rejected because callers could not make progress
through a large valid result. Backend-neutral numeric row cursors were rejected
because they leak storage identity. A generic predicate or relationship query
language was rejected in favor of named, typed domain operations.

## Standards impact

OpenAssetIO relationship queries also return pages and opaque page tokens. The
shape is compatible with a later adapter, but PostProject tokens and
relationship values are its own contract and are not OpenAssetIO tokens or
trait data. No media, metadata, provenance, or identifier standard assigns
semantics to local query pagination.

## Consequences

Large callers can bound allocation and resume queries uniformly on every
surface. Dependency traversal exposes incomplete results instead of hiding a
reached bound. Existing whole-set `dependents()` and `jobs()` calls are replaced
before 1.0; the changelog identifies the paginated replacements. Every new
production-sized query must use this pattern and have an index and scale test
appropriate to its ordering and predicates.
