# Fingerprints, verification, and inventory

[Resolution](media-resolution.md) answers *where* content can be reached. This
guide covers the related question of *what* the content is: recording that a
file's bytes changed, checking stored fingerprints against the files on disk,
finding media on storage that the production does not know yet, and recording
technical facts about a file.

All of these follow one rule: reading the filesystem never changes the
production by itself. A check reports; only an explicit transaction records.

## Record a new fingerprint observation

A fingerprint is the current observation of an object's content, not a
write-once attribute. When a host knows that a file was replaced in place — a
re-rendered plate, a re-conformed audio file — it records a new observation for
the resource. The previous value moves to history, and the change is one
revision event.

A representation fingerprint is derived from its resources' fingerprints. A new
resource observation therefore marks each owning representation for
recomputation until its recomputed fingerprint is recorded too; until then,
[artifact evaluation](artifacts-and-staleness.md) treats the affected knowledge
as pending rather than current.

```{code-variants} fingerprint-observation
```

Recording an identical value is a no-op and creates no revision, so a host may
safely record after every verification.

## Verify content during resolution

By default resolution trusts known locators whose files exist. Verification
additionally recomputes the fingerprints of content found at known locators and
reports a mismatch as evidence, so replaced or corrupted files are noticed.
Verification reads every byte it checks and is opt-in for that reason:

```{code-variants} verify-resolution
:::{no-variant} c
Content verification is not exposed through the C ABI. `pp_production_resolve_asset`
checks known locators and searches mapped roots without recomputing
fingerprints; use the CLI `media resolve --verify` or the Rust adapter.
:::
:::{no-variant} cpp
Content verification is not exposed through the C++ wrapper. `Production::resolve`
checks known locators and searches mapped roots without recomputing
fingerprints; use the CLI `media resolve --verify` or the Rust adapter.
:::
:::{no-variant} python
Content verification is not exposed through the Python binding.
`Production.resolve` checks known locators and searches mapped roots without
recomputing fingerprints; use the CLI `media resolve --verify` or the Rust
adapter.
:::
```

A verification mismatch is a report. Record the new observation, as shown
above, only after the integration has decided the new bytes are correct.

## Inventory storage

An inventory scan walks the mapped media roots and classifies what it finds:
known resources that are online, partial, missing, or changed since they were
recorded; new media that no representation references yet; duplicate and
ambiguous relink candidates; and roots that are unmapped or unavailable on this
machine. It never changes the production. An
optional sidecar cache lets repeated scans skip fingerprinting unchanged files;
the cache is machine-local and holds no production knowledge.

```{code-variants} inventory-scan
:::{no-variant} c
Inventory scanning is not exposed through the C ABI. Run `postproject media
inventory` from a host tool, or resolve individual assets with
`pp_production_resolve_asset`.
:::
:::{no-variant} cpp
Inventory scanning is not exposed through the C++ wrapper. Run `postproject
media inventory` from a host tool, or resolve individual assets with
`Production::resolve`.
:::
:::{no-variant} python
Inventory scanning is not exposed through the Python binding. Run `postproject
media inventory` with `subprocess`, or resolve individual assets with
`Production.resolve`.
:::
```

## Record technical inspection

The optional inspection adapter runs `ffprobe` as a subprocess and records a
bounded technical summary of the container and its streams as one structured
metadata assertion in the `https://postproject.org/ns/technical-media/1`
vocabulary, attached to the representation. A missing `ffprobe` is reported as a capability gap, not an
error, and PostProject never ships or downloads the executable.

```{code-variants} media-inspection
:::{no-variant} c
Inspection is not exposed through the C ABI. A host that already inspects media
can record its own findings as typed [metadata](metadata-vocabularies.md) with
`pp_transaction_add_metadata_value`.
:::
:::{no-variant} cpp
Inspection is not exposed through the C++ wrapper. A host that already inspects
media can record its own findings as typed [metadata](metadata-vocabularies.md)
with `Transaction::addMetadataValue`.
:::
:::{no-variant} python
Inspection is not exposed through the Python binding. A host that already
inspects media can record its own findings as typed
[metadata](metadata-vocabularies.md) with `Transaction.add_metadata`.
:::
```

Inspection results are observations of one file at one time. They do not
replace the fingerprint as evidence of identity.
