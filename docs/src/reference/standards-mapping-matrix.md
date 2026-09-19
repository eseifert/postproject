# Standards mapping matrix

This matrix states design intent, not normative compliance.

| External concept | PostProject representation | Mapping strength | Adapter status |
|---|---|---|---|
| SMPTE UMID | External identifier on an asset or representation | Intended lossless opaque preservation | Helper deferred |
| IPTC VMH property | Vocabulary, property, and typed metadata value | Intended lossless for supported value shapes | Mapping package deferred |
| W3C PROV Entity | Asset or representation, depending on context | Conceptual | Export adapter deferred |
| W3C PROV Activity | Activity with input and output edges | Strong conceptual mapping | Export adapter deferred |
| OTIO external reference | Identifier and locator resolution | Partial | Media linker deferred |
| OTIO rational time | Rational-time value | Intended exact | Adapter deferred |
| FIMS business-media location | Representation plus locator | Conceptual | Adapter deferred |
| AAF source/content ID | External identifier | Intended opaque preservation | Adapter deferred |
| C2PA claim/assertion | Metadata/provenance mapping plus a separate trust layer | Partial | Trust integration deferred |

The authoritative specifications, not this table, define each external concept.
