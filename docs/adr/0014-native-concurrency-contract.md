# ADR 0014: Native concurrency contract

- Status: Accepted
- Date: 2026-09-21

## Context

Production and transaction handles are not safe for concurrent access. Callers must
serialize externally, and a reentrant call returns `PP_ERROR_CONFLICT` because the interior
borrow is already taken.

That contract does not survive contact with an editing application. Editors load, inspect,
and fingerprint media on worker threads while a UI thread reads the same production, so a
host integration meets this on its first day rather than as a late optimization.

Three properties of the current implementation shape the choice:

- `pp_production_resolve_asset` holds its borrow across the whole resolution, including
  media-root traversal and candidate hashing. The critical section today is bounded by
  filesystem work, not by database access.
- The SQLite connection sets a five-second busy timeout and uses the default rollback
  journal. Multiple connections to one production therefore read concurrently, while a
  writer excludes readers. Write-ahead logging would relax that, but it is unreliable on the
  network storage that production files plausibly live on, so it is not assumed here.
- A production permits one open transaction at a time, and the import flow already performs
  fingerprinting before staging. Transactions are intended to be short.

## Decision

The production handle becomes internally synchronized. Concurrent calls on one handle are
safe and block rather than returning a conflict. The handle may be moved between threads.

Transaction handles remain caller-serialized. A transaction is a stateful sequence with one
writer by construction, sharing one across threads has no useful meaning, and the expensive
preparation work already happens outside it.

Two constraints make that contract honest rather than nominal:

- No long-running operation may hold the production lock. Resolution reads the resources,
  locators, and media roots it needs, releases the lock, performs filesystem discovery and
  verification, and assembles results from that snapshot. A lock held across I/O converts a
  correctness problem into a latency problem and calls it solved.
- A panic inside a locked section must not leave the handle permanently unusable. Lock
  state is recovered at the boundary, where `catch_unwind` already translates panics.

Opening a production more than once remains supported and is the documented answer for a
host that needs reads to avoid per-handle serialization during commit. Staging does not hold
the production lock; commit does, so reads on the *same* production state wait while it
persists. A separately opened handle does not share that in-process lock, though SQLite file
locking still applies. Hosts are not required to pool handles, and nothing here obliges them
to.

## Consequences

The common integration shape — one handle, several threads — works without the host building
a connection pool in each binding. That removes an obstacle at exactly the point where
integration friction is most expensive.

Reads on a single handle serialize. For short database reads this is unimportant; for a host
performing a bulk import inside one transaction it is visible, and the documented remedy is a
second handle rather than a finer-grained lock.

Blocking replaces an immediate conflict error for handle contention. `PP_ERROR_CONFLICT`
retains its meaning for transaction lifecycle misuse, such as a second open transaction.

The choice is additive. A later move to a per-thread or pooled model remains possible,
because synchronizing the handle does not prevent opening a production several times. The
reverse would not hold: publishing a caller-pooled contract first would be difficult to
withdraw.

The C header, `docs/abi-policy.md`, the C++ wrapper, and the Python binding must state the
same contract. Documenting it in one of them is how a binding quietly acquires a different
one.
