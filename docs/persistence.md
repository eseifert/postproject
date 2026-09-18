# Persistence

Each project is one SQLite database. The backend enables foreign keys and disables
trusted-schema features on every connection. A five-second busy timeout turns
brief lock contention into bounded waiting rather than an immediate failure.

## Schema version 1

Version 1 stores a singleton project record plus assets, representations,
fingerprints, locations, and media roots. Public identities are 16-byte UUID
values; SQLite row numbers are never exposed. Constraints enforce ID lengths,
enumeration ranges, non-empty fingerprint values, and referential integrity.
Indexes support the first-iteration access patterns: representations by asset,
locations by representation, and enabled media roots by priority.

## Migrations and durability

`PRAGMA user_version` identifies the current schema, while `schema_migrations`
records every applied numbered migration and its timestamp. Each migration runs
inside an immediate SQLite transaction. A failed statement therefore leaves both
the prior schema and version intact. Opening a newer unsupported schema fails
without modifying it.

Project creation reserves a new file without overwriting any existing path, runs
migrations, then inserts project identity and metadata in one transaction. Normal
SQLite transaction durability applies. Backup tooling should copy a closed project
or use SQLite's online backup API once that API is exposed; copying only the main
file while a project is open may omit WAL state.

The backend currently builds a bundled SQLite for reproducible developer and CI
builds. SQLite errors are wrapped as domain storage or migration errors rather than
becoming part of the public contract.

## Domain transactions

Media imports insert the asset, original representation, optional fingerprint,
and initial location inside one explicit deferred SQLite transaction. Media roots
participate in the same transaction boundary. Commit and rollback close the
transaction; repeated close attempts return a conflict. Dropping an open
transaction uses SQLite rollback semantics, so partially staged imports never
become visible.
