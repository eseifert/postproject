# Standards policy

Before adding a canonical scheme, vocabulary mapping, cardinality rule, or
normalization behavior, consult the current authoritative specification. Do not
derive normative behavior from a blog post or memory.

Reviews should ask:

- Does the model preserve the external concept without silently changing it?
- Is the mapping lossless, partial, or only conceptual, and is that documented?
- Does an open-world concept use an extensible identifier instead of a frozen
  enum?
- Can unknown fields survive without the relevant schema package installed?
- Does the feature accidentally turn PostProject into a standards engine,
  timeline model, trust system, or network client?

Long-lived decisions belong in an ADR before implementation is finalized.
