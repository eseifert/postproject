# Metadata vocabulary registry

The Rust core includes a deliberately small registry of optional metadata
hints. It is not an allow-list and is never required to store, read, or query a
metadata assertion. Unknown vocabulary and property strings remain valid and
round-trip exactly.

| Vocabulary | Exact identifier | Built-in property hints |
| --- | --- | --- |
| IPTC Video Metadata Hub 1.7 JSON | `https://iptc.org/std/videometadatahub/recommendation/iptc-vmhub-1.7-schema.json` | `title`, `keywords` |
| Dublin Core elements | `http://purl.org/dc/elements/1.1/` | `title`, `subject` |
| XMP Basic | `http://ns.adobe.com/xap/1.0/` | `CreateDate`, `CreatorTool` |
| EBUCore | `urn:ebu:metadata-schema:ebucore` | vocabulary reference only |
| PostProject metadata | `https://postproject.org/ns/metadata/` | reserved; no native properties currently required |

Definitions can expose accepted PostProject value kinds, single or repeated
cardinality, a label and description, an authoritative reference, optional
local validation, and aliases from mapping profiles. Applications opt in by
looking up a definition and calling its validation method. Persistence does
not call it automatically.

The IPTC hints record selected XMP and EBUCore spellings from the Video
Metadata Hub mapping table. These aliases describe correspondence only. They
do not turn PostProject into an XMP or EBUCore validator and do not claim that
conversion is always lossless.

Authoritative references:

- [IPTC Video Metadata Hub 1.7](https://iptc.org/standards/video-metadata-hub/)
- [IPTC Video Metadata Hub mapping table](https://www.iptc.org/std/videometadatahub/recommendation/IPTC-VideoMetadataHub-mapping-Rec_1.7.html)
- [Dublin Core Metadata Element Set](https://www.dublincore.org/specifications/dublin-core/dces/)
- [Adobe XMP namespaces](https://developer.adobe.com/xmp/docs/xmp-namespaces/)
- [EBU Tech 3293](https://tech.ebu.ch/publications/tech3293)

## Application-specific namespaces

An application should use a stable namespace rooted in a domain it controls,
for example `https://editor.example/metadata/`. It can store those assertions
without registering the namespace with PostProject. A registry hint is useful
only when shared labels, validation, or mappings justify maintaining one.
