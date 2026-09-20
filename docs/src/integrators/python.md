# Python quickstart

The Python package uses the installed public C ABI through the standard
library's `ctypes` module. It does not build or import Rust code.

Point the binding at an exact native library:

```sh
export POSTPROJECT_LIBRARY=/opt/postproject/lib/libpostproject.so
```

Applications may instead pass `library_path=` to `Production.create` or
`Production.open`. The binding resolves that explicit path and does not search
the working directory or modify the platform loader path.

```python
from postproject import OriginIdentity, Production

with Production.create("production.pproj", "Documentary") as production:
    with production.transaction(
        origin=OriginIdentity("com.example.editor", "0.4.0"),
        message="Import camera original",
    ) as transaction:
        asset_id = transaction.import_media(
            "rushes/A001.mov", display_name="Camera A"
        )

    assert production.contains_asset(asset_id)
```

A transaction context commits only after a clean exit. An exception rolls it
back. `close()` is idempotent for production and transaction handles, and a
finalizer is a fallback for handles that were not closed explicitly.

The current high-level surface covers production lifecycle, transactions,
original-media import, revision context, and asset existence. Identifier,
metadata, provenance, revision-feed, compound-media, and resolution wrappers
remain under development. The generated low-level declaration table already
covers every function and struct in the current ABI.
