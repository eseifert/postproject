# Standards mapping matrix

This matrix states design intent, not normative compliance.

| External concept | PostProject representation | Mapping strength | Adapter status |
|---|---|---|---|
| SMPTE UMID | External identifier on an asset or representation | Intended lossless opaque preservation | Helper deferred |
| IPTC VMH property | Vocabulary, property, and typed metadata value | Intended lossless for supported value shapes | Mapping package deferred |
| W3C PROV Entity | Asset or representation, depending on context | Conceptual | Export adapter deferred |
| W3C PROV Activity | Activity with input and output edges | Strong conceptual mapping | Export adapter deferred |
| W3C PROV specialization | Stable representation plus a revision/fingerprint edge snapshot | Conceptual; export must mint a specialized Entity | Export adapter deferred |
| OpenAssetIO Entity Reference | Versioned host-object binding | Manager boundary | Read-only Manager implemented |
| OpenAssetIO locatable content | Representation, resource, and locator resolution | Strong conceptual mapping | Read-only Manager implemented |
| OpenAssetIO-MediaCreation image collection | Image-sequence content structure and frame range | Conceptual; evolving traits require versioned adapter review | Image and frame-range traits implemented |
| OTIO external reference | Representation binding resolved through OpenAssetIO | Partial | Runnable upstream-linker demonstration |
| OTIO image-sequence reference | Image-sequence descriptor and rational rate | Intended strong mapping | Blocked by upstream linker's ExternalReference-only support |
| OTIO rational time | Rational-time value retained by OTIO | Intended exact | Demonstration verifies no transformation |
| FIMS business-media location | Representation, resource, and locator | Conceptual | Adapter deferred |
| AAF source/content ID | External identifier | Intended opaque preservation | Adapter deferred |
| IMF related resources | Package content structure | Conceptual | Adapter deferred |
| C2PA claim/assertion | Metadata/provenance mapping plus a separate trust layer | Partial | Trust integration deferred |

The authoritative specifications, not this table, define each external concept.
