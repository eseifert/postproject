# Bounded queries and cursors

Production-sized queries return stable keyset pages instead of complete lists.
Choose a page size from 1 through 1,000 and pass the returned opaque cursor to
the next call. Keep the query root, filters, page size, and traversal bounds
unchanged while following a cursor.

A cursor is a weakly consistent continuation, not a durable snapshot or a
revision-feed position. A later page sees the current durable state after its
key. Use the [revision feed](revision-feed.md) when an integration needs to
observe changes rather than scan current state.

## Traverse dependencies

Forward queries start at a representation. Reverse queries start at an asset
or representation and return the representations that depend on it. Both
directions require a maximum depth and a visited-representation bound. Each
match includes its shortest observed depth.

```{code-variants} dependency-queries
```

`traversal_truncated` and `next_cursor` answer different questions. A truncated
traversal stopped at a graph bound and may omit matches. A next cursor means
another output page exists within the graph that was actually traversed.

## Filter and page jobs

Job queries accept optional exact state and open-world kind filters. This
example requests two jobs, then drains one-item pages using the same filters on
every call:

```{code-variants} job-query-pages
```

The C cursor borrows its result-set handle and must be copied before releasing
that handle. C++, Python, and Rust return owned cursor values. The CLI emits the
cursor as `next_cursor` in its JSON page object.

Passing a cursor to another named query, root, filter set, page size, or
traversal bound is an invalid argument. Treat cursor contents as private and do
not parse or synthesize them.
