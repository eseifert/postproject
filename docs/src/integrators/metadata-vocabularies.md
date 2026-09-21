# Metadata vocabularies

Metadata properties are identified by an exact vocabulary string and an exact
vocabulary-local property string. Do not rewrite either string during import.
Use a standard namespace when a standard already defines the concept; use a
stable application-owned namespace only for genuinely application-specific
data.

The core Rust API exposes `VocabularyId`, `PropertyId`, `MetadataProperty`,
`MetadataValue`, `MetadataField`, and `MetadataAssertion`. The SQLite production
API supports:

- appending a repeated value;
- atomically replacing every ordered value of a property;
- removing a property;
- reading one property or every assertion on a target;
- finding every assertion with an exact vocabulary/property pair.

All writes belong to an explicit production transaction. A failed operation or
rollback leaves no partial assertions.

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
  title

postproject metadata remove production.pproj asset "$ASSET_ID" \
  https://iptc.org/std/videometadatahub/recommendation/iptc-vmhub-1.7-schema.json \
  title
```

JSON output is explicitly tagged with value types. Decimal coefficients are
strings so JSON consumers do not lose precision. Binary values use hexadecimal
text. Lists and structured fields are recursive and ordered.

Python uses typed immutable values and keyed reads:

```python
from postproject import MetadataLanguageString, MetadataProperty

title = MetadataProperty(
    "https://iptc.org/std/videometadatahub/recommendation/iptc-vmhub-1.7-schema.json",
    "title",
)
with production.transaction() as transaction:
    transaction.add_metadata(
        asset_id, title, MetadataLanguageString("Interview", "en-US")
    )

assertions = production.metadata[asset_id]
matching = production.metadata_by_property[title]
```

The decoder preserves all current ABI value kinds, including exact decimals
and rationals, bytes, ordered lists and structures, and typed object references.
The current mutation ABI accepts plain and language-tagged strings; broader
typed C and Python writes remain outstanding.

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

The typed domain model, optional vocabulary registry, SQLite persistence, Rust
production API, C traversal, typed CLI surface, and Python traversal are
implemented. Activity metadata is writable after the activity is created in
the same or an earlier transaction. The C++ typed wrapper and general typed
writes through the C ABI and Python remain outstanding.

See [standards boundaries](../concepts/standards-boundaries.md) and the
[mapping matrix](../reference/standards-mapping-matrix.md) for the intended
relationship to IPTC Video Metadata Hub, EBUCore, XMP, and other standards.
