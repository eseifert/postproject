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

Ordered spans and packages are added the same way from a list of file members.
Each member has an open-world role and is required unless marked optional; the
order of an ordered-parts list is significant.

## Read the stored structure

After commit, enumerate representations rather than retaining a private host
index. Each representation exposes:

- the content-structure kind;
- ordered members, open-world roles, and requiredness;
- the compact sequence descriptor;
- resources and their fingerprints;
- resource locators; and
- representation fingerprints separately from resource fingerprints.

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
sidecars. Recognition is not yet exposed through the C ABI, C++ wrapper, or
Python binding; those surfaces can still create every recognized structure
explicitly with the operations above.

AVCHD recognition assigns open-world PostProject roles for essence,
clip-information, playlist, and navigation members. These labels describe the
adapter's preservation model; they do not claim conformance validation or
interpret vendor metadata.
