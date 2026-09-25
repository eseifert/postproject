# PostProject Python binding

This package wraps the installed PostProject C ABI with standard-library
`ctypes`. Set `POSTPROJECT_LIBRARY` to the absolute shared-library path, or pass
an explicit library path when opening a production. Python 3.11 or newer is
required.

Tagged GitHub releases include a pure-Python wheel and separate native archives
for Linux, macOS, and Windows. Install the wheel, unpack the matching native
archive, and point `POSTPROJECT_LIBRARY` at its shared library. The wheel does
not search loader paths or bundle a platform binary.

The package is pre-1.0 and tracks the current PostProject ABI without backward
compatibility guarantees.

```python
from pathlib import Path

from postproject import OriginIdentity, Production

media = Path(
    "/opt/postproject/share/doc/postproject/examples/fixtures/sample-media.dat"
)

with Production.create("production.pproj", "Documentary") as production:
    with production.transaction(
        origin=OriginIdentity("com.example.importer"),
        message="Import camera original",
    ) as transaction:
        asset_id = transaction.import_media(media, display_name="Camera A")

    assert asset_id in production.assets

    for representation in production.representations[asset_id]:
        print(representation.structure_kind, representation.resources)

    for representation in production.resolutions[asset_id]:
        print(representation.availability, representation.resources)

    revision = production.latest_revision
    assert revision is not None
    for event in production.revision_events[revision.id]:
        print(event.position, event.payload)

    page = production.jobs(limit=100)
    while True:
        for job in page.items:
            print(job.id, job.state)
        if page.next_cursor is None:
            break
        page = production.jobs(limit=100, cursor=page.next_cursor)
```

Production and transaction handles support deterministic `close()` and context
manager cleanup. A clean transaction context commits; an exception rolls back.
Releasing an unfinished transaction also discards its staged mutations.
Production operations may run concurrently from multiple threads and serialize
inside the native handle; `close()` must not overlap them. Transaction instances
must remain on one caller-controlled execution path. Open the production again
when reads should use a separate native handle during a commit.
Revision summaries, representation structures, resources, locators, fingerprints,
and semantic event payloads are copied Python values; they remain valid after the
temporary native result handles are released. Resolution results likewise copy
the complete nested candidate, evidence, issue, and missing-frame details before
releasing their native result handle.
Query cursors are opaque and must be reused with the same page size, filters,
root, and traversal bounds that produced them.
