# ADR 0019: Media inspection boundary

- Status: Accepted
- Date: 2026-09-23

## Context

Real-media import needs container, stream, rate, duration, timecode, and embedded
metadata observations. Implementing parsers in PostProject would duplicate a
mature media stack and enlarge the untrusted-input surface. Linking FFmpeg
libraries directly would add platform packaging and licensing constraints to a
library distributed as `MIT OR Apache-2.0`.

Inspection is useful evidence, but it must not be required to preserve media
identity or import a production.

## Decision

Technical media inspection is an adapter in `postproject-media`. The first
adapter invokes `ffprobe` as a subprocess and requests bounded JSON output. No
FFmpeg library is linked, and `postproject-core` gains no FFmpeg dependency or
technical-media fields.

The executable path is caller-configurable. A missing executable is reported as
an unavailable capability, distinct from invalid media and subprocess failure.
Import continues without inspection when that capability is unavailable.

The adapter limits captured stdout and stderr, rejects oversized or excessively
nested values, validates numeric strings before conversion, and treats the
subprocess output as untrusted. It does not execute a shell.

Extracted observations are translated to values in the existing metadata model
under documented vocabulary identifiers. Unknown tags are preserved only when a
caller opts into an explicit application namespace; PostProject does not mint
substitute names for another organization's fields.

## Alternatives considered

- **Link FFmpeg libraries.** This offers a richer in-process API but increases
  native dependency, ABI, distribution, and licensing complexity.
- **Write format parsers.** This duplicates specialist work and creates a large
  security-sensitive maintenance burden.
- **Put technical fields in the core model.** Container and codec vocabularies
  evolve independently and fit the extensible metadata mechanism already in use.

## Consequences

Systems without `ffprobe` retain complete import and resolution behavior but do
not receive technical metadata or the evidence derived from it. Subprocess
startup has a cost, so bulk callers may schedule inspection separately. Tests use
a deterministic fake executable and do not depend on FFmpeg being installed.
