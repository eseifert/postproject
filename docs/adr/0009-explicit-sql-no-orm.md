# ADR 0009: Explicit SQL without an ORM

- Status: Accepted
- Date: 2026-09-19

## Decision

SQLite persistence uses explicit SQL and migrations behind domain-shaped core
contracts. No generic repository or ORM is introduced.

## Consequences

Schema, indexes, query plans, migrations, and corruption handling remain visible
and testable. The storage boundary does not pretend that a future PostgreSQL
backend has SQLite-identical behavior, and database concepts do not leak into
the domain model.
