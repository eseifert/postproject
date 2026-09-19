# External identifiers

An external identifier consists of an extensible scheme string, an opaque value,
and an optional qualifier. Multiple identifiers, including multiple values from
one scheme, may be attached to an object.

Core validation is intentionally small: text must be non-empty, bounded, and
free of NUL characters so every public language boundary can transport it.
PostProject preserves unknown schemes and values exactly.
Applications may opt into a scheme-specific validator when they need to verify
syntax; storing an identifier never triggers a registry or network lookup.

Scheme identifiers are strings rather than a closed enum so new standards and
application namespaces do not require an ABI redesign. Canonical constants for
known schemes are added only after their authoritative specifications have been
checked.
