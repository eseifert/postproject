# ADR 0013: Namespace policy

- Status: Accepted
- Date: 2026-09-21

## Context

PostProject mints identifier strings in several places, and they are persisted — in
production files, in other applications' project files, and in exported metadata. Once
external data carries them, changing them is expensive.

Four kinds exist today:

- the application identifier scheme `org.postproject.application`;
- resource roles and activity kinds such as `postproject:essence` and
  `postproject:transcode`, whose `namespace:local` form reserves `postproject` to this
  project;
- the portable host-object binding, serialized as `postproject:v1:<production>:<kind>:<uuid>`;
- vocabulary identifiers, where the registry stores other bodies' namespaces exactly as
  published — `urn:ebu:metadata-schema:ebucore`, `http://purl.org/dc/elements/1.1/`, the
  IPTC VMH schema URL.

The project now controls the domain `postproject.org`. That changes what these strings can
honestly claim. A reverse-DNS namespace is conventionally backed by a domain its author
controls, so `org.postproject.application` becomes properly grounded rather than merely
asserted. It also raises a question the bare `postproject:` prefix did not have to answer
while no domain existed: `postproject:` is shaped like a URI scheme but is not a registered
one, and `urn:postproject:` would require an IANA-registered URN namespace identifier under
RFC 8141.

The standards policy in `docs/src/contributors/standards-policy.md` requires consulting the
authoritative specification before adopting normative behavior, and the project does not
claim compliance it has not earned. An unregistered scheme presented as though it were
registered would breach that.

## Decision

PostProject uses two namespace forms rooted in the domain it controls:

- **Published vocabularies and properties** use HTTPS identifiers below
  `https://postproject.org/ns/`. They can dereference to their own documentation and require
  no private URI-scheme or URN registration.
- **Application-owned external identifiers** use the scheme identifier
  `https://postproject.org/id/application`.
- **Resource roles and activity kinds/roles** use compact reverse-DNS tokens beginning with
  `org.postproject:`, followed by the existing vocabulary-local token. The Rust types provide
  the semantic context, so the shared prefix does not need a type component.
- **Portable host-object bindings** are HTTPS identifiers of the form
  `https://postproject.org/ref/v1/<production UUID>/<object kind>/<object UUID>`. Object kinds
  and UUID spelling remain as defined by ADR 0011.

HTTPS identifiers are ordinary URLs under a controlled domain, not claims to a registered
custom scheme. The compact reverse-DNS token is not a URI scheme and must not be presented as
one. Third parties should use a namespace rooted in a domain they control.

Identifiers owned by other organizations are stored exactly as published. PostProject does
not rewrite them beneath `postproject.org` or create substitute names for them.

The former `postproject:` and `org.postproject.application` development strings are removed.
The pre-1.0 project has no external consumers, so parsers and registries accept only the new
canonical forms and production fixtures are updated rather than migrated.

## Consequences

Namespaces become verifiable rather than asserted, and a reader encountering a PostProject
identifier can find out what it means.

The host-binding form is settled before the first integration experiment persists references.
Future changes require an explicit versioned format and migration decision.
