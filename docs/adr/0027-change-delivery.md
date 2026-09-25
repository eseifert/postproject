# ADR 0027: Change delivery

- Status: Accepted
- Date: 2026-09-26

## Context

ADR 0007 gives every committed transaction one revision and a pull contract:
`latest_revision`, `changes_since(sequence, limit)`, and `events_for_revision`.
A host that wants to react to changes must poll that contract on a timer. An
editor that polls too slowly shows stale state, one that polls too quickly
spends its time reading an unchanged journal, and neither learns about a commit
made by a worker in another process any sooner than its next poll.

Production files are local SQLite databases shared by several processes, such
as an editor, a CLI worker, and an OpenAssetIO host. There is no daemon, and
none is wanted before a service backend exists. Commits from another process
are invisible to in-process notification, and platform file-change APIs differ
in semantics and reliability, especially on network storage.

The native surfaces must not call foreign code from library-owned threads.
ADR 0014 makes a production handle internally synchronized and forbids holding
its lock across long-running work, which includes blocking until someone else
commits.

## Decision

### Wait primitive

A *revision waiter* is created from an open production and waits for the first
revisions after a caller-supplied sequence. One wait takes that sequence, a page
limit from 1 through 1,000, and a timeout from zero through 60 seconds. It
returns exactly one outcome:

- **revisions** — a non-empty ascending page, identical to
  `changes_since(sequence, limit)` at the moment the wait observed it;
- **timed out** — no revision after the sequence existed within the timeout;
- **closed** — the production the waiter was created from was closed; or
- **cancelled** — the waiter was cancelled.

A zero timeout checks once without blocking. Closed and cancelled are terminal:
every later wait on that waiter returns the same outcome immediately, so a
cancellation or close that races with the start of a wait is never lost.
Cancelling is the one waiter operation that may be called from any thread while
another thread waits.

The waiter owns a dedicated read connection to the production file and never
uses the production's own connection or lock while blocked. It detects commits
in two ways:

- A commit through the production the waiter was created from signals every
  waiter created from it, which then reads immediately.
- Commits from other processes, and from other production handles in the same
  process, are detected by polling `PRAGMA data_version` on the waiter's
  connection. The poll interval starts at 5 ms and doubles to at most 100 ms
  while nothing changes. The journal is read only when the data version
  changed, or once at the start of each wait.

The data version, not the in-process signal, is authoritative: a signal only
shortens the wait. A reader that finds the database busy treats the poll as
unchanged and retries at the next interval instead of blocking past its
timeout. Opening a waiter verifies that the file still holds the same
production.

Timeouts are measured with a monotonic clock inside the adapter. Tests assert
on outcomes, never on elapsed time.

### No callbacks across the C ABI

The C ABI exposes the waiter handle and nothing that calls back. A waiter is
created from a production, waits, is cancelled, and is released. Releasing a
production handle closes every waiter created from it. Waiter handles are
caller-serialized except for cancellation; a concurrent second wait on one
waiter returns `PP_ERROR_CONFLICT`. A waiter never touches the production
handle after it has been created, so a production may be released while
another thread waits on a waiter without violating the handle contract.

The C++ wrapper and Python build observers on the waiter. An observer owns one
thread, waits with the maximum timeout, loads the matching revisions and their
events, and calls the application's callback on that thread. Stopping an
observer cancels its waiter and joins the thread. No foreign callback ever runs
on a thread the library created, so ADR 0014 gains an addition rather than a
revision.

### Filtered revision pages

A consumer may fetch revisions after a sequence that contain at least one event
of a non-empty set of event types. Event types are the payload-free
discriminants of the semantic event catalog. The page returns the matching
revisions in ascending order, with all of their events, plus a *through
sequence*: every matching revision with a sequence greater than the request and
at most the through sequence is in the page. A full page ends at its last
revision. A short page ends at the newest revision that existed when the page
was read, or at the requested sequence if that is newer. The through sequence
is the next cursor, so a consumer can advance past long runs of unrelated
revisions without reading them. The cursor keeps the meaning it has for
`changes_since`: a production-local revision sequence.

Schema 13 adds a derived table keyed by event kind and revision sequence. A
trigger fills it from every inserted event and migration backfills it from the
journal; deleting a revision cascades to it. A filtered page reads at most one
page of keys per requested event type, so its cost is proportional to the page
and the number of requested types, not to the length of the journal.

## Alternatives considered

A background thread that invokes registered callbacks was rejected because it
runs foreign code on a library thread and makes lock ordering the host's
problem. Filesystem notification (inotify, FSEvents, ReadDirectoryChangesW)
was rejected because its behavior differs per platform and is unreliable on
network filesystems; it could later shorten the poll without changing the
contract. SQLite's update and commit hooks observe only the connection that
registered them, so they cannot see other processes. Write-ahead logging would
not change detection and is not assumed on network storage (ADR 0014).

Waiting on the production handle itself was rejected because stopping one
observer would require closing the production for every other user of it.
Waits without a timeout were rejected because a blocked native thread must
always return control to its owner. Filtering inside the wait was rejected: a
wait that returns on any revision followed by a filtered page gives the same
result with one simpler primitive.

## Standards impact

No media, metadata, provenance, or identifier standard assigns semantics to
local change delivery. OpenAssetIO has no change-notification contract for a
manager to implement. W3C PROV and the event meanings of ADR 0007 are
unchanged.

## Consequences

A host learns about another process's commit within one poll interval of the
commit, and about a commit through its own production immediately, without a
daemon or platform-specific code. Every waiter holds one extra SQLite
connection. Waiting is read-only and never takes a write lock, so waiters do not
delay writers.

Event meaning does not change. Consumers still treat events as an invalidation
guide and re-query current state. A service backend may deliver the same
outcomes over a network stream later, behind the same wait contract.
