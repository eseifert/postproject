# Contributor guide

The domain crate is storage- and framework-neutral. SQLite implements
domain-shaped contracts, filesystem inspection lives in the media crate, and
the CLI and public C ABI are adapters. Rust layouts and backend handles never
cross the ABI.

A change to schema, public API, ABI, or domain meaning must include:

1. an updated or new architecture decision record;
2. a standards-impact check;
3. domain and audience-appropriate documentation;
4. persistence changes and fixtures where relevant;
5. coordinated Rust, C, C++, Python, and example updates for affected surfaces;
6. invariant-focused tests and fuzzing where relevant;
7. a changelog entry.

Use explicit SQL rather than an ORM, preserve unknown external data, keep
transactions atomic, and make corruption return errors rather than panics.
