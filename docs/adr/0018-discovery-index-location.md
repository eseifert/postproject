# ADR 0018: Discovery index location

- Status: Accepted
- Date: 2026-09-23

## Context

Walking a large media root and reading filesystem metadata on every resolution is
expensive. The observations gathered by a scan are nevertheless machine-local:
paths, modification times, and reachability describe one workstation at one
moment. Storing them in the production database would make a portable knowledge
file carry disposable state from whichever machine scanned most recently.

The index must improve repeated scans without becoming a second authority for
media identity or production membership.

## Decision

The persistent discovery index is a sidecar cache outside the `.pproj` file. A
caller selects its location, allowing desktop applications to use their normal
per-user cache directory and command-line workflows to use an explicit path.

The cache records enough bounded filesystem facts to avoid revisiting unchanged
files: logical root name, relative path, size, modification time, and cached
fingerprint evidence where it has been computed. It is versioned independently
of the production schema and associates entries with a production identity and
the supplied root mapping.

Scanning remains authoritative over the cache. Changed directory or file facts
invalidate affected entries. A missing, incompatible, truncated, or corrupt
cache is discarded logically and rebuilt by a full scan. It never prevents the
production from opening or changes a scan result category.

The index does not import files, confirm locators, or mutate production state.
Deletion must affect performance only.

## Alternatives considered

- **Tables in the production database.** This gives transactional convenience
  but mixes local, rebuildable observations with portable production knowledge.
- **One global content index.** It may be useful later, but introduces lifecycle,
  privacy, and cross-production policy beyond this release.
- **Memory-only caching.** It cannot make a second process or a later invocation
  materially cheaper.

## Consequences

Applications own cache placement and cleanup. The cache format carries no public
compatibility promise and can be rebuilt after upgrades. Tests must prove result
equivalence with no cache, a warm cache, and a corrupt cache. Remote and shared
indexes remain separate future adapters.
