# ADR 0015: Availability semantics and collection fingerprints

- Status: Accepted
- Date: 2026-09-22

## Context

Two questions block sequence and package resolution.

**What does it mean for a resource to be present at a known locator?** The current check tests
whether the locator addresses a file, so an intact image sequence — which addresses a
directory — resolves as offline and its recorded frame exceptions are never consulted.

**What is a collection resource's fingerprint computed over?** A resource standing for a whole
image sequence has never been created outside tests, so the question has never been answered.

Both are settled in adjacent standards, and the convergence is strong enough to adopt rather
than invent.

BagIt (RFC 8493) separates two operations by name. A bag is **complete** when every file listed
in every manifest is present — existence only. It is **valid** when it is complete *and* every
checksum has been verified. Digital Cinema Packaging draws the same line across two documents:
the Packing List carries per-asset hashes, the ASSETMAP carries locations, and validators
report structural cross-referencing separately from hash verification. That is the distinction
PostProject already makes for single files between a known locator being available and a
candidate being verified by fingerprint.

On identity, the pattern is a digest over a canonical list of member digests with location
excluded. Git tree objects hash entry modes, names, and child object hashes, never contents
directly and never absolute paths. The C2PA collection data hash, added in specification 2.1,
holds per-file hashes keyed by relative path and takes the base path as a parameter at
hash and verify time rather than storing it, so identity survives relocation. Notably it keeps
per-member digests rather than rolling them into a single root; aggregate identity arises
because the assertion is itself hashed into the manifest.

OpenTimelineIO contributes by declining the question. It defines no availability state and
instead defines a consumption policy — `missing_frame_policy` of error, hold, or black — which
places the decision about what to do with a gap in the consumer rather than the model.

## Decision

**Presence and verification are separate tiers.** A resource is present at a known locator when
its declared members are present. Verifying that those members still carry the expected content
is a distinct, more expensive assertion, reported through fingerprint evidence as it already is
for single files. "Online" never implies "verified".

**A resource's declared members depend on its content structure.** For a single-resource
structure the declared member is the file the locator addresses, which is the current
behavior. For an image sequence the declared members are the filenames produced by the
pattern across the frame domain, less the frames already recorded as missing. For ordered parts
and packages the declared members are the required members; optional members follow ADR 0003
and do not reduce availability.

**Presence of a sequence is answered by enumerating the locator's directory, not by testing
each frame.** Expected names are computed from the pattern and compared against one listing.
This is bounded by the directory size rather than the frame count, which is what makes the
check affordable for a sequence of any length.

**A missing declared member yields `Partial` with diagnostics, not `Offline`.** PostProject
reports which members or frames are absent and does not decide what the consumer should do
about them. Substituting, holding, or failing is the host's policy.

**Collection fingerprints are computed over member content, not over structure alone.** A digest
of a sequence descriptor is a digest of metadata: two unrelated sequences sharing a prefix,
suffix, padding, frame range, and rate would collide. A collection fingerprint therefore
combines the canonical structure with digests of member content, and excludes object identities
and locators so it survives relocation, as ADR 0003 already requires.

**A sequence resource is fingerprinted by sampling members, and the sampling is part of the
algorithm identity.** Hashing every frame is not affordable and hashing none is not evidence.
A deterministic subset of frames is hashed, following the large-file fingerprint precedent of
sampling fixed regions rather than reading the whole file. Coverage is recorded, and a sampled
collection fingerprint never claims the confidence of a complete one.

Ordering follows ADR 0003 unchanged: preserved for ordered parts, irrelevant for packages.

## Consequences

An intact sequence resolves online, a sequence with a recorded gap resolves partial, and both
answers cost one directory listing. The five availability states become observable rather than
nominal.

Sampled collection fingerprints are identity evidence of bounded strength, which matches how
sampled single-file fingerprints already behave. A caller that needs certainty verifies
members, which is the validity tier rather than the presence tier.

Verifying frames against the filesystem beyond presence — detecting that a frame was deleted or
rewritten after import — remains filesystem-scanning work scheduled for a later release.
This decision defines what that work refines, not a substitute for it.

The two tiers give integrations a vocabulary that already exists elsewhere. A host that
understands BagIt's complete-versus-valid distinction, or a DCP validator's split between
structural and hash checks, will recognize the same split here.
