# ADR 0011: Host-object binding

- Status: Accepted
- Date: 2026-09-20

## Context

An application that embeds PostProject keeps its own project, session, or scene file. That
file must be able to refer back to PostProject objects, and the reference must survive the
ordinary things users do to project files.

PostProject already models the outward direction: an external identifier lets an
application record its own identifier against a PostProject object. The inward direction is
unspecified. Without it, each host invents its own convention, and two hosts opening the
same production cannot agree on what a reference means.

The reference must survive at least:

- saving and reopening the host project;
- duplicating or saving-as the host project;
- copying a host project to another location;
- importing media from one host project into another;
- opening the host project on another machine;
- moving the PostProject database.

## Decision

A host persists a `ProductionId` together with the referenced object's kind and UUID. It may
also persist a human-readable fallback such as a display name or last known locator, but the
fallback never participates in identity. An object UUID is production-scoped; the complete
`(ProductionId, object kind, object UUID)` tuple is the portable reference.

The canonical serialized form is an opaque, versioned ASCII value:

```text
postproject:v1:<production UUID>:<object kind>:<object UUID>
```

Object-kind tokens are `production`, `asset`, `representation`, `resource`, and `activity`.
UUIDs use lowercase hyphenated RFC 9562 text. Parsers reject unknown versions, unknown kinds,
extra fields, and non-canonical UUID spellings. This is a PostProject interchange value, not a
registered URI scheme; hosts must preserve it as opaque text. Advisory fallback text is stored
separately and is deliberately absent from the serialized identity.

The host remains authoritative for its private project, session, scene, timeline, and UI
state. The referenced PostProject production remains authoritative for portable production
knowledge. A disagreement is reported and requires an explicit rebind; neither side silently
rewrites the other.

PostProject records stable host object identifiers through the existing external-identifier
mechanism. They are lookup aids and cross-application assertions, not ownership claims, and
do not gain global uniqueness merely because a host supplies them. No second host-link domain
model is introduced.

Duplicating or saving-as a host document does not duplicate the production or its objects.
Both host documents may legitimately retain the same references. A host that gives the new
document or its private objects distinct identities records new external identifiers as
needed; it does not replace PostProject object identity.

When the production is unavailable, the host preserves the opaque reference and any fallback
instead of deleting or fabricating production knowledge. It degrades to its native behavior
and cached or host-owned information until the production can be reopened.

## Consequences

A host can hold a durable reference without adopting PostProject as its project format, and
independent integrations agree on what a reference means. Moving the database does not change
the tuple, while moving media remains a separate locator-resolution concern.

Integration profiles can share one binding format and differ only in how the production is
located, owned, and made available. The core formatting and parsing helper standardizes the
tuple without making fallback text authoritative.
