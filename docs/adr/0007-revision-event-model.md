# ADR 0007: Revision and event model

- Status: Accepted
- Date: 2026-09-20

## Decision

Every successful mutating transaction creates exactly one durable revision with
a monotonically increasing local sequence. Domain changes, the revision, and
semantic change events commit atomically. Rollback and failed commits create no
revision.

Consumers pull deterministic pages of revisions and events. Events identify
what changed so consumers can re-query current state; they are not serialized
Rust values or SQL row diffs.

Each revision stores a stable ID, a positive production-local sequence, the
transaction ID, commit time, an optional integrating-tool origin, and an
optional bounded message. The origin names the application or process that
performed the mutation; it is not an authenticated person or authorization
claim.

The semantic event catalog covers:

- imported assets, added representations, resources, representation-resource
  membership, locators, and media roots;
- added and removed external identifier attachments;
- added/replaced and removed metadata properties; and
- created activities plus their input and output edges;
- changed resource and representation fingerprint domains;
- replaced representation dependency observations; and
- requested, claimed, renewed, released, succeeded, failed, and cancelled jobs.

ADR 0021 adds fingerprint-observation events. Re-observing the same bytes in
the same domain is a no-op and does not advance the feed.

ADRs 0023 and 0024 add job-lifecycle and dependency-observation events. Job
events identify the job for re-query but never expose its claim token.

The pull contract is `latest_revision`, `changes_since(sequence, limit)`, and
`events_for_revision`. Revision pages and event lists are ordered ascending by
their local sequence and stable event position respectively. Page size is
explicitly bounded.

## Consequences

The journal supports observation and future synchronization work. It is not an
undo stack, collaboration protocol, or distributed merge system.

SQLite persists revisions and their semantic events in the same database
transaction as the domain mutations. Empty transactions, rollbacks, and failed
mutations do not advance the feed. Consumers should treat event payloads as an
invalidation/re-query guide rather than as a replayable replacement for current
production state.
