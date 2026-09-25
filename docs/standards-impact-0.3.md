# Release 0.3 standards-impact check

Release 0.3 filesystem recognition adds no canonical identifier scheme,
metadata vocabulary, normalization rule, or standards conformance claim.
Numbered sequence and span recognition is based on filename syntax. Sidecar
recognition preserves files without interpreting their contents.

The AVCHD adapter recognizes only the conventional `PRIVATE/AVCHD/BDMV`
directory shape needed to preserve a card as one package. Member roles use the
open-world `org.postproject.avchd` namespace and remain adapter descriptions,
not normative AVCHD terms. Unknown files are not discarded from a production
because the recognizer never mutates an existing import.

Technical metadata in the 0.3 release series retains source keys or maps them
through documented PostProject-owned vocabulary terms; it does not imply that
`ffprobe` output is an authoritative standards interpretation.
