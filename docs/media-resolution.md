# Media resolution

## Fingerprints

Fingerprint format version 1 uses BLAKE3. Files up to and including 1 MiB store
the standard 32-byte BLAKE3 hash of every content byte under algorithm identifier
`pp-blake3-full-file`. Larger files store a digest over a domain separator, file
size, and 64 KiB regions at deterministic beginning, middle, and end offsets under
`pp-blake3-sampled-regions`.

The sampled format is designed for relocation candidate verification, not as a
collision-proof or adversarial content identifier. The resolver must expose it as
partial-fingerprint evidence. A caller can later request a full-file hash when
stronger verification is required.

Fingerprinting rejects symbolic links and non-regular files. It compares file size
and modification time before and after reading, failing rather than persisting a
result when the file appears to change during calculation.

## Resolution policy

Resolution checks known locations first and scans configured roots only when
necessary. Traversal is deterministic, does not follow symlinks, defaults to a
depth limit of 64 and an entry limit of 100,000, and reports a structured error
result when a bound or filesystem operation prevents a safe answer.

Discovery, cheap file-size filtering, and fingerprint verification are separate
stages. Full hashes produce exact resolution; sampled fingerprints produce
probable resolution. If no fingerprint exists, a matching filename is required
and file size strengthens the evidence. Equally credible candidates produce
`Ambiguous` and require explicit confirmation. Confirmation adds a new location
inside a project transaction; the resolver itself never mutates project state.

Current scans are intentionally uncached. Overlapping roots are de-duplicated by
canonical file URI, but each resolve operation walks enabled roots afresh. A later
filesystem index can replace discovery without changing result semantics.

Rust callers receive the domain `Resolution` values directly. Native C callers
receive an opaque resolution set and inspect state, candidate confidence, URI,
and both result-level and candidate-level evidence through bounded accessors.
The C++17 wrapper copies the same information into value objects. Native
confirmation remains a separate explicit transaction operation.
