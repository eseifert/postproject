# Provenance model rules

Changes to activity provenance must preserve these invariants:

- activity kinds and edge roles are bounded, namespaced, open-world identifiers;
- a completed activity has at least one output and may have zero inputs;
- identical input or output edges are rejected;
- one representation cannot appear on both sides of an activity;
- fan-in, fan-out, and cross-asset processing remain supported;
- finish time cannot precede start time when both are present;
- tool and agent detail is bounded and contains no implicit network behavior;
- activity parameters use typed metadata rather than a second JSON property bag;
- a transaction cannot introduce a generation cycle;
- failed creation leaves no partial activity or edge rows;
- stored corruption returns a storage error rather than panicking;
- enumeration and graph traversal are deterministic and cycle-safe.

The core crate owns these semantics and contains no SQL. SQLite stores activities
and input/output edges in normalized tables. Read paths use set-oriented edge
queries; do not replace them with one query per activity. The recursive ancestry
and descendant queries use duplicate-eliminating traversal so malformed cycles
terminate, although normal writes reject cycles before commit.

Do not store `derived-from` as the only provenance fact or infer proxy, revision,
variant, or alternative relationships from an activity. Those concepts require
explicit knowledge. Likewise, activity provenance does not execute work and does
not claim cryptographic authenticity.

Schema, graph-invariant, or public API changes require an ADR review, rollback
and malformed-data tests, updates to every implemented adapter, and an entry in
the changelog.
