# Persistence

Each project is one SQLite database. The backend enables foreign keys and disables
trusted-schema features on every connection. A five-second busy timeout turns
brief lock contention into bounded waiting rather than an immediate failure.
SQLite's per-connection value-length limit is reduced to 16 MiB before migrations
or queries run. This bounds allocations for strings, blobs, and result rows read
from an untrusted project file while leaving ample room for project metadata.

## Schema version 1

The current development schema stores a singleton project record plus assets,
representations, content structures, resources, memberships, locators, typed
fingerprints, media roots, metadata assertions, and external identifiers.
Image-sequence descriptors and their known missing frames are stored compactly;
a regular sequence does not require one resource row per frame. Public
identities are 16-byte UUID values; SQLite row numbers are never exposed.

Constraints enforce ID lengths, enumeration ranges, bounded text and blobs,
non-empty fingerprint values, and referential integrity. Indexes support
representations by asset, resources by representation, locators by resource,
external identifiers by target and exact scheme/value, metadata by target or
property, and enabled media roots by priority.

External identifiers and metadata assertions use polymorphic typed targets.
SQLite triggers clean up attachments because one target column cannot carry
foreign keys to several domain tables.

## Migrations and durability

`PRAGMA user_version` identifies the current schema, while `schema_migrations`
records every applied numbered migration and its timestamp. Each migration runs
inside an immediate SQLite transaction. A failed statement therefore leaves both
the prior schema and version intact. Opening a newer unsupported schema fails
without modifying it. Earlier development layouts are unsupported. The current
initial migration is the canonical schema because no external project files
were published for the discarded layouts.

Project creation reserves a new file without overwriting any existing path, runs
migrations, then inserts project identity and metadata in one transaction. Normal
SQLite transaction durability applies. Backup tooling should copy a closed project
or use SQLite's online backup API once that API is exposed; copying only the main
file while a project is open may omit WAL state.

The backend currently builds a bundled SQLite for reproducible developer and CI
builds. SQLite errors are wrapped as domain storage or migration errors rather than
becoming part of the public contract.

## Domain transactions

Media imports insert the asset, representation, content structure, resources,
typed fingerprints, memberships, and initial locators inside one explicit
deferred SQLite transaction. Media roots, metadata assertions, and
external-identifier attachments/removals participate in the same transaction
boundary. Commit and rollback close the transaction; repeated close attempts
return a conflict. Dropping an open transaction uses SQLite rollback semantics,
so partially staged changes never become visible.
