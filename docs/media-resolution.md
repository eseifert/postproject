# Media resolution

## Fingerprints

Resource fingerprint format version 1 uses BLAKE3. Files up to and including
1 MiB store the standard 32-byte BLAKE3 hash of every content byte. Larger files
store a strategy-versioned digest over the file size and 64 KiB regions at
deterministic beginning, middle, and end offsets.

The sampled format is designed for relocation candidate verification, not as a
collision-proof or adversarial content identifier. The resolver must expose it as
partial-fingerprint evidence. A caller can later request a full-file hash when
stronger verification is required.

Fingerprinting rejects symbolic links and non-regular files. It compares file
size and modification time before and after reading, failing rather than
persisting a result when the file appears to change during calculation. These
are resource fingerprints; structure-aware representation fingerprints are a
separate domain and may use different strategies.

## Resolution policy

Resource resolution checks known locators first and scans configured roots only
when necessary. Traversal is deterministic, does not follow symlinks, defaults
to a depth limit of 64 and an entry limit of 100,000, and reports a structured
error result when a bound or filesystem operation prevents a safe answer.

Discovery, cheap file-size filtering, and fingerprint verification are separate
stages. Full hashes produce exact resolution; sampled fingerprints produce
probable resolution. If no fingerprint exists, a matching filename is required
and file size strengthens the evidence. Equally credible candidates produce
`Ambiguous` and require explicit confirmation. Confirmation adds a new locator
for the selected resource inside a production transaction; the resolver itself
never mutates production state.

Representation availability is then aggregated from its content structure.
Every required member online is `Online`; a mix of online and offline required
members is `Partial`; no resolvable required members is `Offline`; and an
ambiguous required member makes the representation `Ambiguous`. Optional
package members produce diagnostics without reducing availability. Known
missing image-sequence frames make an otherwise online sequence `Partial`, with
the exact frames retained in the diagnostic.

Current scans are intentionally uncached. Overlapping roots are de-duplicated by
canonical file URI, but each resolve operation walks enabled roots afresh. A later
filesystem index can replace discovery without changing result semantics.

Rust callers receive `ResourceResolution` values and aggregate them into a
`RepresentationResolution`. The CLI emits one representation result containing
ordered resource results and availability issues. The C ABI exposes the same
nested shape through index-checked accessors, and the C++ wrapper copies it into
owned `RepresentationResolution` values. Native confirmation remains a separate
explicit transaction operation.
