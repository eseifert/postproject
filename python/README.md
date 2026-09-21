# PostProject Python binding

This package wraps the installed PostProject C ABI with standard-library
`ctypes`. Set `POSTPROJECT_LIBRARY` to the absolute shared-library path, or pass
an explicit library path when opening a production. Python 3.11 or newer is
required.

The package is pre-1.0 and tracks the current PostProject ABI without backward
compatibility guarantees.

```python
from postproject import Production

with Production.create("production.pproj", "Documentary") as production:
    with production.transaction(
        origin="example.importer", message="Import camera original"
    ) as transaction:
        transaction.add_media_root("rushes", "Camera originals")
        asset_id = transaction.import_media(
            "rushes/A001.mov", display_name="Camera A"
        )

    assert asset_id in production.assets

    for representation in production.resolutions[asset_id]:
        print(representation.availability, representation.resources)

    revision = production.latest_revision
    assert revision is not None
    for event in production.revision_events[revision.id]:
        print(event.position, event.payload)
```

Production and transaction handles support deterministic `close()` and context
manager cleanup. A clean transaction context commits; an exception rolls back.
Releasing an unfinished transaction also discards its staged mutations.
Revision summaries and semantic event payloads are copied Python values; they
remain valid after the temporary native result handles are released. Resolution
results likewise copy the complete nested candidate, evidence, issue, and
missing-frame details before releasing their native result handle.
