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

Resolution will check known locations first and scan configured roots only when
necessary. Discovery, cheap file-fact filtering, and fingerprint verification
will be separate stages. Results expose evidence and candidates; equally credible
candidates produce `Ambiguous` and require explicit confirmation.
