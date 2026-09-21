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

## CLI inspection

The demonstrator can add text, list recursively typed values, find a property,
and remove all values of a property:

```sh
postproject metadata add-text production.pproj asset "$ASSET_ID" \
  http://iptc.org/std/videometadatahub/1.0 title "Interview" \
  --language en-US

postproject --json metadata list \
  production.pproj asset "$ASSET_ID"

postproject --json metadata find production.pproj \
  http://iptc.org/std/videometadatahub/1.0 title

postproject metadata remove production.pproj asset "$ASSET_ID" \
  http://iptc.org/std/videometadatahub/1.0 title
```

JSON output is explicitly tagged with value types. Decimal coefficients are
strings so JSON consumers do not lose precision. Binary values use hexadecimal
text. Lists and structured fields are recursive and ordered.

The CLI currently creates plain and language-tagged text values. Complex value
input will use a documented typed file format rather than requiring unreadable
shell quoting.

Python uses typed immutable values and keyed reads:

```python
from postproject import MetadataLanguageString, MetadataProperty

title = MetadataProperty(
    "http://iptc.org/std/videometadatahub/1.0", "title"
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
typed writes remain outstanding.

## Availability

The typed domain model, SQLite persistence, Rust production API, C traversal,
CLI read surface, and Python traversal are implemented. Activity metadata is
writable after the activity is created in the same or an earlier transaction.
The C++ typed wrapper and general typed writes through the C ABI, CLI, and
Python remain outstanding.

See [standards boundaries](../concepts/standards-boundaries.md) and the
[mapping matrix](../reference/standards-mapping-matrix.md) for the intended
relationship to IPTC Video Metadata Hub, EBUCore, XMP, and other standards.
