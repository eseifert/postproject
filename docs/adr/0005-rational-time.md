# ADR 0005: Rational time

- Status: Proposed
- Date: 2026-09-19

## Decision

Canonical time values use integers and a validated rational rate. Time ranges
contain a rational start and duration. Persistence does not use floating point,
and conversion or arithmetic is checked for overflow.

## Consequences

The values can map cleanly to OTIO-like rational time and future AAF/FIMS/IMF
adapters. They do not introduce tracks, clips, timecode formatting, or timeline
ownership into PostProject.
