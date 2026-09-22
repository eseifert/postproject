# Compound-media integration

A host creates one representation for a sequence, span, or package. It should
not import every member as an unrelated asset. The representation owns a
content structure; its resources carry content evidence and one or more
locators.

Python can add a compact image sequence to an existing asset:

```python
from postproject import ImageSequenceInput, RepresentationKind

with production.transaction() as transaction:
    sequence_id = transaction.add_image_sequence_representation(
        asset_id,
        RepresentationKind.DERIVED,
        ImageSequenceInput(
            directory="renders/shot010",
            prefix="shot010.",
            suffix=".exr",
            padding=4,
            start=1001,
            end=1100,
            step=1,
            rate_numerator=24000,
            rate_denominator=1001,
            missing_frames=(1042,),
        ),
    )

sequence = next(
    item
    for item in production.representations[asset_id]
    if item.id == sequence_id
)
```

The C++17 wrapper exposes the same transaction operation as
`addImageSequenceRepresentation`. Ordered spans and packages use
`addOrderedPartsRepresentation` and `addPackageRepresentation` with
`FileResourceInput` values. In C, the corresponding functions are
`pp_transaction_add_image_sequence_representation`,
`pp_transaction_add_ordered_parts_representation`, and
`pp_transaction_add_package_representation`.

After commit, enumerate representations rather than retaining a private host
index. C uses `pp_production_representations` and the representation-set
accessors. C++ uses `Production::representations`, and Python uses
`production.representations[asset_id]`. Each surface exposes:

- the content-structure kind;
- ordered members, open-world roles, and requiredness;
- the compact sequence descriptor;
- resources and their fingerprints;
- resource locators; and
- representation fingerprints separately from resource fingerprints.

Resolution returns one aggregate availability value plus resource results and
issues. Missing sequence frames are sorted individual frame numbers. Optional
package members may produce issues but do not reduce availability. Never choose
one ambiguous candidate in integration code; present the candidates to the user
and persist only an explicit confirmation.
