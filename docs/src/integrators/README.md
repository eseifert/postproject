# Integrator guide

PostProject exposes a public C ABI with opaque handles. The C++17 wrapper is a
RAII layer over that ABI, and installed consumers do not need Rust or Cargo.
Until an explicit stability milestone, pin an exact release or commit and expect
coordinated API, ABI, schema, CLI, and binding changes.

Key integration rules:

- treat PostProject IDs as internal object identities, not industry IDs;
- preserve external scheme/value text exactly;
- never choose an ambiguous relink candidate silently;
- perform mutations through explicit transactions;
- advance revision cursors only after processing a complete revision;
- release owned C handles with their documented release function;
- do not infer “revision”, “variant”, or “alternative” relationships from a
  processing activity.

An identifier attachment in C uses a typed object reference and remains pending
until commit:

```c
pp_object_ref_t target = {PP_OBJECT_ASSET, asset_id};
pp_transaction_add_external_identifier(
    tx, &target, "com.example.camera.serial", "A-0007", NULL, &error);
pp_transaction_commit(tx, &error);
```

The equivalent C++ wrapper copies values out of the C result-set handle:

```cpp
postproject::ObjectRef target{postproject::ObjectKind::asset, asset_id};
tx.addExternalIdentifier(
    target, {"com.example.camera.serial", "A-0007", std::nullopt});
tx.commit();
auto identifiers = production.externalIdentifiers(target);
```

Python uses the typed ID itself as the object reference:

```python
from postproject import ExternalIdentifier

identifier = ExternalIdentifier(
    "com.example.camera.serial", "A-0007"
)
with production.transaction() as transaction:
    transaction.add_external_identifier(asset_id, identifier)

identifiers = production.external_identifiers[asset_id]
matches = production.objects_by_external_identifier[
    identifier.scheme, identifier.value
]
```

The [metadata guide](metadata-vocabularies.md), [provenance
guide](provenance.md), and [revision feed guide](revision-feed.md) document the
implemented cross-language surfaces. The root README contains the shortest
native build and C example.
