# Media roots and resolution

Media moves: a production is copied to another workstation, a volume is
mounted at a different path, or a folder is reorganized. PostProject keeps
asset and representation identity stable while storage locations change, and
resolution answers where the content can be reached *now*.

## Register a logical media root

A media root is a portable, production-wide name such as `rushes`. It does not
store a machine path. Each machine supplies the local directory for a root name
at resolution time, so the same production resolves correctly on differently
mounted systems.

```{code-variants} media-root
```

Roots have an optional label and a priority; lower priorities are searched
first. A root can later be disabled, re-enabled, or removed.

## Resolve an asset

Resolution checks the known locators of every resource and, where content is
missing, searches the mapped roots for credible candidates. It reports one
aggregate availability per representation (online, partial, offline,
ambiguous, or error), the state of each resource, scored candidates with their
evidence, and availability issues such as missing sequence frames.

The example maps the `rushes` root to this machine's directory after the
imported file has been moved there:

```{code-variants} resolve-asset
```

Resolution is read-only. It never changes locators, even when exactly one
candidate matches exactly. An unmapped root is reported as unmapped evidence
rather than an error, and an unreadable mapping as unavailable; other roots are
still searched.

## Confirm a candidate

Only an explicit confirmation makes a candidate durable. The integration
decides which candidate to confirm: a person picks one of several plausible
candidates, or a policy accepts a single exact match. Confirmation adds a
locator for the resource in a transaction; the stored identity is unchanged.

A candidate found under a mapped root carries `media_root_relation` evidence
that names the logical root. The example confirms such a candidate under that
root, so the locator records the portable root name but never the local
directory. The CLI `media resolve --confirm` does the same automatically.
[Locator and media-root queries](bounded-queries.md) report that recorded root.

```{code-variants} confirm-locator
```

Never confirm one of several candidates automatically. Present them, with
their confidence and evidence, and let the user choose.
