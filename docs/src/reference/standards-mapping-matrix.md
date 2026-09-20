# Standards mapping matrix

This matrix states design intent, not normative compliance.

| External concept | PostProject representation | Mapping strength | Adapter status |
|---|---|---|---|
| SMPTE UMID | External identifier on an asset or representation | Intended lossless opaque preservation | Helper deferred |
| IPTC VMH property | Vocabulary, property, and typed metadata value | Intended lossless for supported value shapes | Mapping package deferred |
| W3C PROV Entity | Asset or representation, depending on context | Conceptual | Export adapter deferred |
| W3C PROV Activity | Activity with input and output edges | Strong conceptual mapping | Export adapter deferred |
| OpenAssetIO Entity Reference | Adapter-generated reference to a PostProject object | Manager boundary | Manager deferred |
| OpenAssetIO locatable content | Representation, resource, and locator | Strong conceptual mapping | Manager deferred |
| OpenAssetIO-MediaCreation image collection | Image-sequence content structure | Conceptual; evolving traits require versioned adapter review | Manager deferred |
| OTIO external reference | Identifier and representation resolution | Partial | OpenAssetIO integration deferred |
| OTIO image-sequence reference | Image-sequence descriptor and rational rate | Intended strong mapping | Validation deferred |
| OTIO rational time | Rational-time value | Intended exact | Adapter deferred |
| FIMS business-media location | Representation, resource, and locator | Conceptual | Adapter deferred |
| AAF source/content ID | External identifier | Intended opaque preservation | Adapter deferred |
| IMF related resources | Package content structure | Conceptual | Adapter deferred |
| C2PA claim/assertion | Metadata/provenance mapping plus a separate trust layer | Partial | Trust integration deferred |

The authoritative specifications, not this table, define each external concept.
