# Consuming the revision feed

Use the revision feed when an integration needs to refresh caches, update a UI,
or observe changes made by another tool sharing the same production file. Store the
last fully processed sequence as the local cursor.

## Polling safely

1. Call `changes_since(cursor, limit)` with a bounded page size.
2. Process revisions in returned order.
3. Load each revision's events in position order.
4. Re-query objects needed by the integration.
5. Advance the local cursor only after the whole revision is processed.
6. Repeat until a page is shorter than the requested limit.

Persisting the cursor after each complete revision gives at-least-once
processing after a consumer crash. Handlers should therefore tolerate seeing a
revision again. A cursor is meaningful only for the production that produced it.

## CLI inspection

The demonstrator exposes the same pull model:

```sh
postproject --json revisions latest production.pproj
postproject --json revisions since production.pproj --after 0 --limit 100
postproject --json revisions events production.pproj REVISION_ID
```

Mutation commands identify their revisions with the `postproject-cli` origin,
the package version, and a short operation message.

## Rust

Storage backends implement the domain-shaped `ProductionRead` contract:

```rust
let page = production.changes_since(cursor, 100)?;
for revision in page {
    for event in production.events_for_revision(revision.id())? {
        handle(event)?;
    }
    cursor = revision.sequence();
}
```

Set optional context before staging mutations when the integrating process
should be identifiable:

```rust
let origin = OriginIdentity::new("editor-host", Some(build), None)?;
transaction.set_revision_context(RevisionContext::new(
    Some(origin),
    Some("Import camera original".to_owned()),
)?)?;
```

## C

The C ABI uses caller-owned result-set handles. Strings returned from an
accessor borrow the owning handle and must be copied before it is released.

```c
pp_revision_set_t *page = NULL;
pp_error_t *error = NULL;
if (pp_production_changes_since(production, cursor, 100, &page, &error) != PP_OK) {
    /* inspect and release error */
}

for (uint64_t i = 0; i < pp_revision_set_count(page); ++i) {
    pp_uuid_t revision_id, transaction_id;
    uint64_t sequence;
    int64_t committed_at;
    const char *origin_name, *origin_version, *origin_uri, *message;
    pp_revision_set_get(page, i, &revision_id, &sequence, &transaction_id,
                        &committed_at, &origin_name, &origin_version,
                        &origin_uri, &message, &error);

    pp_revision_event_set_t *events = NULL;
    pp_production_revision_events(production, &revision_id, &events, &error);
    for (uint64_t j = 0; j < pp_revision_event_set_count(events); ++j) {
        pp_revision_event_t event;
        pp_revision_event_set_get(events, j, &event, &error);
        /* switch on event.kind; unused fields are zero or NULL */
    }
    pp_revision_event_set_release(events);
    cursor = sequence;
}
pp_revision_set_release(page);
```

Call `pp_transaction_set_revision_context` before commit to attach the optional
origin and message.

## C++

The C++17 wrapper copies results into values and converts the tagged C event
record into a `std::variant`:

```cpp
for (const auto &revision : production.changesSince(cursor, 100)) {
  for (const auto &event : production.revisionEvents(revision.id)) {
    std::visit(handle_event, event.payload);
  }
  cursor = revision.sequence;
}
```

`Transaction::setRevisionContext` accepts an optional `OriginIdentity` and
message. Result handles and borrowed strings remain internal to the wrapper.
