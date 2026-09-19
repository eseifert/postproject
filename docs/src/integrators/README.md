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
auto identifiers = project.externalIdentifiers(target);
```

The [metadata guide](metadata-vocabularies.md) documents the implemented typed
model, storage limits, and CLI inspection surface. Metadata access through the
C ABI, C++ wrapper, and future Python binding will be documented when those
public surfaces land.

The root README contains the shortest native build and C example. Detailed
provenance, revision-feed, and Python examples will be added with those public
surfaces.
