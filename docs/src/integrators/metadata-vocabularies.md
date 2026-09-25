# Metadata vocabularies

Metadata properties are identified by an exact vocabulary string and an exact
vocabulary-local property string. Do not rewrite either string during import.
Use a standard namespace when a standard already defines the concept; use a
stable application-owned namespace only for genuinely application-specific
data.

## Add and read metadata

A metadata assertion attaches a typed value to a production, asset,
representation, resource, or activity under an exact vocabulary and property.
Every surface can:

- append a value, so a property can hold several ordered values;
- remove every value of a property from a target;
- read every assertion on a target; and
- find every assertion that uses an exact vocabulary and property, as one
  [bounded page](bounded-queries.md) at a time and optionally restricted to an
  exact scalar value.

The Rust storage API can additionally replace every ordered value of a property
atomically. The example adds a language-tagged title to an asset and reads it
back from both directions:

```{code-variants} metadata
```

All writes belong to an explicit production transaction. A failed operation or
rollback leaves no partial assertions.

## Typed values

Values are typed rather than stringly encoded: plain and language-tagged text,
signed and unsigned 64-bit integers, exact decimals and rationals, booleans,
timestamps, URIs, opaque bytes, typed object references, and recursively nested
ordered lists and named-field structures. Every surface preserves every value
kind on read. In C, recursive input handles copy their children, so callers can
release intermediate list and structure values immediately after construction.

## CLI input and inspection

The demonstrator can add text or any recursively typed value, list values, find
a property, and remove all values of a property. A typed value uses the same
tagged JSON shape emitted by `--json` output. For example, `contact.json` may
contain:

```json
{
  "type": "struct",
  "fields": [
    {"name": "name", "value": {"type": "string", "value": "Camera department"}},
    {"name": "confidence", "value": {"type": "decimal", "coefficient": "995", "scale": 3}}
  ]
}
```

Write it and inspect it with:

```sh
postproject metadata add production.pproj asset "$ASSET_ID" \
  https://example.com/vocabulary contact contact.json

postproject metadata add-text production.pproj asset "$ASSET_ID" \
  https://iptc.org/std/videometadatahub/recommendation/iptc-vmhub-1.7-schema.json \
  title "Interview" \
  --language en-US

postproject --json metadata list \
  production.pproj asset "$ASSET_ID"

postproject --json metadata find production.pproj \
  https://iptc.org/std/videometadatahub/recommendation/iptc-vmhub-1.7-schema.json \
  title --limit 100

postproject metadata remove production.pproj asset "$ASSET_ID" \
  https://iptc.org/std/videometadatahub/recommendation/iptc-vmhub-1.7-schema.json \
  title
```

`metadata find` returns one page: its JSON output is an object with `items`,
`next_cursor`, and `traversal_truncated`, and `--cursor` continues from
`next_cursor`. Pass `--value-file` with a file holding one tagged scalar value,
such as `{"type": "lang_string", "value": "Interview", "language": "en-US"}`, to
return only assertions with exactly that value.

JSON output is explicitly tagged with value types. Decimal coefficients are
strings so JSON consumers do not lose precision. Binary values use hexadecimal
text. Lists and structured fields are recursive and ordered.

## Technical inspection

The Rust media crate defines a `MediaInspector` adapter boundary and a bounded
`FfprobeInspector` subprocess implementation. Successful results are ordinary
typed assertions under
`https://postproject.org/ns/technical-media/1` with property `inspection`; no
FFmpeg type or dependency enters `postproject-core`. Raw embedded tag keys and
values are represented as ordered key/value structures so unfamiliar tags do
not need to become schema fields.

The CLI reaches this adapter with `media add --inspect` and attaches successful
assertions to the imported representation in the same transaction. The direct
inspection operation is not currently exposed through C, C++, or Python; those
surfaces can read the resulting assertion through their existing metadata
traversal APIs.

`media resolve --verify` reuses a single stored inspection as partial identity
evidence when scoring relocated file candidates. The candidate remains
ambiguous if another credible match exists. Use `--ffprobe PATH` to select the
inspector executable; an unavailable or failed inspector leaves the other
resolver evidence unchanged.

Subprocess output is limited to 8 MiB per stream, execution defaults to a
30-second deadline, JSON and numeric values are parsed without floating point,
and stderr diagnostics are truncated. Missing `ffprobe`, non-zero exit, timeout,
oversized output, and malformed JSON are distinguishable outcomes.

## Optional Rust registry

The core registry supplies a small set of advisory definitions for IPTC Video
Metadata Hub JSON, Dublin Core, XMP Basic, EBUCore, and PostProject-owned
metadata. A property hint can describe accepted value kinds, cardinality,
labels, descriptions, and known mapping aliases. `validate_values` applies
those rules only when an application explicitly calls it.

Registry lookup uses exact identifiers. An absent vocabulary or property is
not an error, and persistence never invokes the registry automatically. This
keeps unknown and application-specific metadata fully round-trippable.

## Availability

The typed domain model, optional vocabulary registry, and SQLite persistence
back every surface. C, Python, Rust, and the CLI read and write every value
kind. The C++ wrapper writes every value kind and reads property queries with
`queryMetadata`, but does not wrap reading every assertion on one target; C++
integrations call `pp_production_metadata` for that. Activity metadata
is writable after the activity is created in the same or an earlier
transaction.

See [standards boundaries](../concepts/standards-boundaries.md) and the
[mapping matrix](../reference/standards-mapping-matrix.md) for the intended
relationship to IPTC Video Metadata Hub, EBUCore, XMP, and other standards.
