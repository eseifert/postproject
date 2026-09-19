# Metadata model rules

Metadata changes must preserve these invariants:

- vocabulary and property identifiers are open strings, not closed enums;
- unknown terms remain readable without a registry;
- repeated assertions are distinct from one list value;
- list order and structured-field order are preserved;
- persistence never uses floating point or unconstrained JSON as the canonical
  form;
- every constructor enforces the public size and nesting limits;
- malformed stored values produce `ErrorKind::Storage`, never a panic;
- metadata remains on its explicit target and is never copied implicitly;
- the private encoding version is checked before any value is decoded.

The deterministic encoding lives in the SQLite crate because it is a storage
choice, not a core domain contract. Do not expose encoded blobs through Rust,
C, C++, CLI, or Python APIs. Public adapters inspect values by type and recurse
through lists and structured fields.

Adding a vocabulary mapping does not make PostProject normative for that
standard. Link to the authoritative vocabulary, document mapping strength, and
preserve source values that the helper does not understand. Do not copy a full
third-party vocabulary into the repository without reviewing its license and
maintenance implications.

Schema, encoding, limit, or public-type changes require an ADR review, malformed
input tests, round-trip tests, migration coverage, and corresponding updates to
every public adapter that already exposes metadata.
