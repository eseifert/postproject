# Consuming the revision feed

Use the revision feed when an integration needs to refresh caches, update a UI,
or observe changes made by another tool sharing the same production file. Every
committed transaction becomes one revision with a production-local sequence
number and an ordered list of semantic events. Store the last fully processed
sequence as the local cursor.

## Polling safely

1. Request the revisions after the cursor with a bounded page size.
2. Process revisions in returned order.
3. Load each revision's events in position order.
4. Re-query objects needed by the integration.
5. Advance the local cursor only after the whole revision is processed.
6. Repeat until a page is shorter than the requested limit.

```{code-variants} revision-feed
```

Persisting the cursor after each complete revision gives at-least-once
processing after a consumer crash. Handlers should therefore tolerate seeing a
revision again. A cursor is meaningful only for the production that produced it.

## Events

Each event has a position within its revision and a typed payload, such as an
imported asset, an added representation or locator, an added or removed
external identifier or metadata property, or a created activity with its edges.
The surfaces expose the same payloads idiomatically:

- C returns a tagged `pp_revision_event_t` record; fields unused by an event
  kind are zero or `NULL`, and strings borrow the event set.
- C++ converts the record into a `std::variant` of event structs.
- Python returns frozen typed values such as `AssetImportedEvent`, suitable for
  `isinstance` dispatch.
- Rust returns `RevisionEventKind` enum values.
- The CLI prints events as JSON with `--json`.

Where the surface offers typed payloads, dispatch on the payload type rather
than on the numeric C event kinds.

## Attributing changes

A transaction may carry an optional *revision context*: an origin identity
(name, version, and URI of the integrating tool) and a short message. Set it
before committing, as shown in
[create a production and import media](first-production.md), so other
consumers of the feed can tell which tool made a change. CLI mutations use the
`postproject-cli` origin with the package version and a short operation
message.
