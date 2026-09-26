# External identifiers

An external identifier consists of an extensible scheme string, an opaque value,
and an optional qualifier. Multiple identifiers, including multiple values from
one scheme, may be attached to an object.

Core validation is intentionally small: text must be non-empty, bounded, and
free of NUL characters so every public language boundary can transport it.
PostProject preserves unknown schemes and values exactly.
Applications may opt into a scheme-specific validator when they need to verify
syntax; storing an identifier never triggers a registry or network lookup.

Identifiers may target assets, representations, resources, or activities. An
activity identifier names the operation itself, such as a render-farm job or
workflow task; it is distinct from the optional identifier of the agent that
performed the activity.

Scheme identifiers are strings rather than a closed enum so new standards and
application namespaces do not require an ABI redesign. The small built-in
registry currently describes:

| Scheme | Stored value | Local check |
| --- | --- | --- |
| `urn:smpte:umid` | the UMID namespace-specific hex value | SMPTE ST 2029 lexical form |
| `urn:isan` | the ISAN namespace-specific value | RFC 4246 lexical form |
| `urn:eidr` | the EIDR prefix and suffix, such as `10.5240:…` | RFC 7972 lexical form |
| `https://postproject.org/id/application` | application-owned opaque text | generic limits only |

These are opt-in syntax hints, not a closed allow-list. Checks do not normalize
case, validate registry assignment, verify every standard checksum, or contact a
network service. See [SMPTE ST 2029](https://pub.smpte.org/doc/st2029/20090310-pub/st2029-2009.pdf),
[RFC 4246](https://www.rfc-editor.org/rfc/rfc4246.html), and
[RFC 7972](https://www.rfc-editor.org/rfc/rfc7972.html) for the authoritative
formats.

## Across public surfaces

This example attaches an application identifier and performs the corresponding
exact lookup:

```{code-variants} external-identifiers
```

Removal names the exact scheme, value, and qualifier; other attachments on the
same object stay in place:

```{code-variants} remove-identifier
```
