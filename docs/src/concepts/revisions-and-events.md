# Revisions and semantic events

A revision is the durable record of one successful mutating transaction. It
gives another application a production-local cursor for discovering changes
without repeatedly scanning the whole production.

Every non-empty committed transaction creates exactly one revision. The domain
changes, revision, and ordered semantic events are atomic: consumers see all of
them or none of them. Empty transactions, rollbacks, and failed mutations do
not advance the revision sequence.

## Revision identity and origin

A revision contains:

- a stable `RevisionId`;
- a positive, monotonically increasing production-local sequence;
- the `TransactionId` that produced it;
- a commit timestamp;
- an optional integrating application/process identity; and
- an optional bounded human-facing message.

The origin describes software such as an editor, ingest service, or automation
worker. It is not an authenticated user identity, authorship proof, or access
control decision.

## Event catalog

Events say which semantic fact changed and provide enough identity for a
consumer to re-query current state.

| Event | Meaning |
| --- | --- |
| `AssetImported` | A logical asset import aggregate was created. |
| `RepresentationAdded` | A representation was attached to an asset. |
| `ResourceAdded` | A storage resource was created. |
| `RepresentationResourceAdded` | A resource entered a representation structure. |
| `LocatorAdded` | A resource locator was added or explicitly confirmed. |
| `LocatorRetired` | A superseded resource locator was removed. |
| `MediaRootAdded` | A resolver search root was added. |
| `MediaRootEnabledChanged` | A resolver search root was enabled or disabled. |
| `MediaRootRemoved` | A resolver search root was removed. |
| `ExternalIdentifierAdded` | An exact identifier attachment was added. |
| `ExternalIdentifierRemoved` | An exact identifier attachment was removed. |
| `MetadataAddedOrReplaced` | Values for one metadata property changed. |
| `MetadataRemoved` | One metadata property was removed. |
| `ActivityCreated` | A production activity was recorded. |
| `ActivityInputAdded` | A consumed-representation edge was recorded. |
| `ActivityOutputAdded` | A produced-representation edge was recorded. |

The catalog is semantic, not a stream of SQL row operations or serialized Rust
objects. Multiple events in one revision preserve their stable transaction
order.

## Pull model

Consumers poll with three operations:

- `latest_revision` discovers the current cursor;
- `changes_since(sequence, limit)` reads a bounded ascending page; and
- `events_for_revision(revision_id)` reads that revision's ordered events.

After receiving an event, a consumer should re-query the relevant object when
it needs current values. Events are an observation and cache-invalidation
mechanism, not a replay log that replaces the production database.

## Deliberate boundaries

The journal is not undo/redo. It does not store inverse operations.

The journal is not itself multi-user collaboration. It has no distributed
merge, base-revision conflict protocol, authenticated authorship, subscription
transport, or remote ordering. Those capabilities can build on the durable
semantic cursor later without changing what existing revisions mean.
