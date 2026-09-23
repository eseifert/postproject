# Iteration 3 standards-impact check

Iteration 3 filesystem recognition adds no canonical identifier scheme,
metadata vocabulary, normalization rule, or standards conformance claim.
Numbered sequence and span recognition is based on filename syntax. Sidecar
recognition preserves files without interpreting their contents.

The AVCHD adapter recognizes only the conventional `PRIVATE/AVCHD/BDMV`
directory shape needed to preserve a card as one package. Member roles use the
open-world `org.postproject.avchd` namespace and remain adapter descriptions,
not normative AVCHD terms. Unknown files are not discarded from a production
because the recognizer never mutates an existing import.

Technical metadata added later in this iteration must likewise retain source
keys or map them through documented PostProject-owned vocabulary terms; it must
not imply that `ffprobe` output is an authoritative standards interpretation.
