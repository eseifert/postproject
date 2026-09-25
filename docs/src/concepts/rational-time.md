# Rational time and ranges

PostProject represents reusable time values without floating point. A
`RationalRate` is a positive, reduced fraction such as `24/1` or
`30000/1001`. A `RationalTime` combines an integer value with that rate. Its
exact duration in seconds is:

```text
value × rate denominator / rate numerator
```

The canonical text forms are `numerator/denominator` for a rate and
`value@numerator/denominator` for a time. For example, 1,001 units at the exact
fractional rate are `1001@24000/1001`, without a rounded decimal rate.

## Comparison and conversion

Ordinary Rust equality compares the stored value and normalized rate. Use
`RationalTime::equivalent` to compare the exact times represented at different
rates. For example, `24@24/1` and `48@48/1` are equivalent.

`RationalTime::rescaled_to` succeeds only when the target rate can represent
the result as an exact integer and the value fits in `i64`. It never rounds.
Comparison, rescaling, and range-end calculation use checked integer
arithmetic and report overflow as a domain error.

## Time ranges

A `TimeRange` is half-open: it contains a start and a non-negative duration at
the same rate. Its canonical text form is `start+duration`, for example:

```text
1001@30000/1001+300@30000/1001
```

`end_exclusive` computes start plus duration with overflow checking.

## Not formatted timecode

Rational time is a mathematical value for exact interchange. It is not a
timeline, edit, track, wall-clock timestamp, SMPTE timecode label, or drop-frame
formatting rule. Those concepts may map to rational values, but their display
and contextual semantics belong in later adapters. PostProject does not infer
them from a rate alone.

## Across public surfaces

Image-sequence creation carries an exact rational rate and an inclusive,
stepped frame range through every public surface:

```{code-variants} image-sequence
```
