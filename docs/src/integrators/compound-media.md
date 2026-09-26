# Compound-media integration

A host creates one representation for a sequence, span, or package. It should
not import every member as an unrelated asset. The representation owns a
content structure; its resources carry content evidence and one or more
locators.

## Add an image sequence

An image-sequence representation is described compactly: directory, filename
prefix and suffix, frame-number padding, first and last frame, frame step, an
exact rational frame rate, and any frames already known to be missing. The
example adds a derived render sequence to an existing asset and reads back the
stored descriptor:

```{code-variants} image-sequence
```

## Add a proxy or other single-file representation

A proxy, an optimized mezzanine, or a derived render is another representation
of the same asset, not a new asset. The example adds a single-file proxy to the
imported asset; the file becomes a new resource with a confirmed locator and a
content fingerprint:

```{code-variants} add-representation
```

The representation kind (original, proxy, optimized, or derived) says how the
representation relates to the asset. Why it exists — which activity produced it
from which input — is recorded separately as [provenance](provenance.md).

## Add ordered parts

A camera recording spanned across several files is one representation whose
members are ordered. Each member carries an open-world role and is required
unless marked optional; the order of the list is significant and preserved:

```{code-variants} ordered-parts
```

## Add a package

A package groups files that belong together without an order, such as an
essence file and its metadata sidecar. Optional members may be missing without
making the representation unavailable:

```{code-variants} package-representation
```

## Read the stored structure

After commit, enumerate representations rather than retaining a private host
index. Each representation exposes:

- the content-structure kind;
- ordered members, open-world roles, and requiredness;
- the compact sequence descriptor and the frames known to be missing;
- resources and their fingerprints;
- resource locators; and
- representation fingerprints separately from resource fingerprints.

The example reads one page of an asset's representations and walks that
structure:

```{code-variants} representation-structure
```

## Availability

[Resolution](media-resolution.md) returns one aggregate availability value
plus resource results and issues. Missing sequence frames are sorted individual
frame numbers. Optional package members may produce issues but do not reduce
availability. Never choose one ambiguous candidate in integration code; present
the candidates to the user and persist only an explicit confirmation.

## Recognition

The Rust media adapter additionally recognizes compound media from filenames
and layouts: numbered image groups, numbered camera spans, same-stem sidecars,
and the checked AVCHD card layout. The CLI reaches the same adapter when
`media add` receives a directory, and with `--recognize-companions` for
sidecars:

```{code-variants} media-recognition
:::{no-variant} c
Recognition is not exposed through the C ABI. Create the recognized structure
explicitly with `pp_transaction_add_image_sequence_representation`,
`pp_transaction_add_ordered_parts_representation`, or
`pp_transaction_add_package_representation`, as shown above.
:::
:::{no-variant} cpp
Recognition is not exposed through the C++ wrapper. Create the recognized
structure explicitly with `addImageSequenceRepresentation`,
`addOrderedPartsRepresentation`, or `addPackageRepresentation`, as shown above.
:::
:::{no-variant} python
Recognition is not exposed through the Python binding. Create the recognized
structure explicitly with `add_image_sequence_representation`,
`add_ordered_parts_representation`, or `add_package_representation`, as shown
above.
:::
```

AVCHD recognition assigns open-world PostProject roles for essence,
clip-information, playlist, and navigation members. These labels describe the
adapter's preservation model; they do not claim conformance validation or
interpret vendor metadata.
