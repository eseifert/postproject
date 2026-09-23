# External identifiers

Attach an industry, vendor, or application identifier to a PostProject object
when another system needs to find that object by its own key. An identifier is
a [scheme, value, and optional qualifier](../concepts/external-identifiers.md);
PostProject stores all three exactly as supplied and never rewrites or
normalizes them.

The example attaches a camera serial number to an asset, reads the identifiers
attached to that asset, and finds every object carrying the exact scheme and
value:

```{code-variants} external-identifiers
```

Attachment is a transactional mutation like any other: it stays pending until
commit and appears in the [revision feed](revision-feed.md). Removing an
identifier requires the same exact scheme, value, and qualifier.

Lookup is exact. A scheme is not a namespace prefix, values are compared
byte-for-byte, and a lookup may return several objects because an external
system can reuse a value. Treat the result as candidates for the integration to
interpret, not as proof of identity.

To refer from a host document *to* a PostProject object, persist a
[host-object binding](host-object-bindings.md) instead; external identifiers
are lookup aids and assertions, not PostProject identity.
