# ADR 0006: Rational time

- Status: Accepted
- Date: 2026-09-19

## Decision

Canonical time values use integers and a validated rational rate. Time ranges
contain a rational start and duration. Persistence does not use floating point,
and conversion or arithmetic is checked for overflow.

Rates are positive and reduced. Cross-rate equivalence is computed by exact
integer cross multiplication. Rescaling succeeds only when the target rate can
represent the value exactly; it never rounds. Canonical text uses
`numerator/denominator`, `value@rate`, and `start+duration` forms.

## Consequences

The values can map cleanly to OTIO-like rational time and future AAF/FIMS/IMF
adapters. They do not introduce tracks, clips, timecode formatting, or timeline
ownership into PostProject.

Structural Rust equality retains the stored rate distinction. Consumers use
the explicit equivalence operation when they need exact cross-rate comparison.
