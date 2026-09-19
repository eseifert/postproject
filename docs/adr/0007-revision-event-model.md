# ADR 0007: Revision and event model

- Status: Proposed
- Date: 2026-09-19

## Decision

Every successful mutating transaction creates exactly one durable revision with
a monotonically increasing local sequence. Domain changes, the revision, and
semantic change events commit atomically. Rollback and failed commits create no
revision.

Consumers pull deterministic pages of revisions and events. Events identify
what changed so consumers can re-query current state; they are not serialized
Rust values or SQL row diffs.

## Consequences

The journal supports observation and future synchronization work. It is not an
undo stack, collaboration protocol, or distributed merge system.
