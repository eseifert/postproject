//! Exact rational time values shared by media structures and adapters.

use std::{cmp::Ordering, fmt, str::FromStr};

use crate::{Error, ErrorKind, Result};

/// A positive, normalized rational rate.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct RationalRate {
    numerator: u32,
    denominator: u32,
}

impl RationalRate {
    /// Creates a reduced positive rate.
    ///
    /// # Errors
    ///
    /// Returns [`ErrorKind::InvalidArgument`] when either component is zero.
    pub fn new(numerator: u32, denominator: u32) -> Result<Self> {
        if numerator == 0 || denominator == 0 {
            return Err(Error::new(
                ErrorKind::InvalidArgument,
                "rational rate components must be greater than zero",
            ));
        }
        let divisor = greatest_common_divisor(numerator, denominator);
        Ok(Self {
            numerator: numerator / divisor,
            denominator: denominator / divisor,
        })
    }

    /// Returns the reduced numerator.
    #[must_use]
    pub const fn numerator(self) -> u32 {
        self.numerator
    }

    /// Returns the reduced denominator.
    #[must_use]
    pub const fn denominator(self) -> u32 {
        self.denominator
    }
}

impl fmt::Display for RationalRate {
    /// Formats the normalized rate as `numerator/denominator`.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}/{}", self.numerator, self.denominator)
    }
}

impl FromStr for RationalRate {
    type Err = Error;

    /// Parses the canonical `numerator/denominator` representation.
    fn from_str(value: &str) -> Result<Self> {
        let (numerator, denominator) = value.split_once('/').ok_or_else(|| {
            Error::new(
                ErrorKind::InvalidArgument,
                "rational rate must use numerator/denominator syntax",
            )
        })?;
        let numerator = numerator.parse::<u32>().map_err(|_| {
            Error::new(
                ErrorKind::InvalidArgument,
                "rational rate numerator must be an unsigned integer",
            )
        })?;
        let denominator = denominator.parse::<u32>().map_err(|_| {
            Error::new(
                ErrorKind::InvalidArgument,
                "rational rate denominator must be an unsigned integer",
            )
        })?;
        Self::new(numerator, denominator)
    }
}

/// An exact integer value measured at a rational rate.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct RationalTime {
    value: i64,
    rate: RationalRate,
}

impl RationalTime {
    /// Creates a rational time value.
    #[must_use]
    pub const fn new(value: i64, rate: RationalRate) -> Self {
        Self { value, rate }
    }

    /// Returns the integer value at this time's rate.
    #[must_use]
    pub const fn value(self) -> i64 {
        self.value
    }

    /// Returns the rate used to interpret the value.
    #[must_use]
    pub const fn rate(self) -> RationalRate {
        self.rate
    }

    /// Compares two time values exactly without floating-point conversion.
    ///
    /// # Errors
    ///
    /// Returns [`ErrorKind::InvalidArgument`] if checked cross multiplication
    /// exceeds the internal comparison range.
    pub fn checked_cmp(self, other: Self) -> Result<Ordering> {
        let left = comparison_product(self.value, self.rate.denominator, other.rate.numerator)?;
        let right = comparison_product(other.value, other.rate.denominator, self.rate.numerator)?;
        Ok(left.cmp(&right))
    }

    /// Returns whether two values denote the same exact time.
    ///
    /// # Errors
    ///
    /// Returns an error under the same conditions as [`Self::checked_cmp`].
    pub fn equivalent(self, other: Self) -> Result<bool> {
        Ok(self.checked_cmp(other)? == Ordering::Equal)
    }

    /// Converts this value to `rate` when the result is an exact integer.
    ///
    /// # Errors
    ///
    /// Returns [`ErrorKind::InvalidArgument`] if the target rate cannot
    /// represent the time exactly or the converted value exceeds [`i64`].
    pub fn rescaled_to(self, rate: RationalRate) -> Result<Self> {
        let numerator = i128::from(self.value)
            .checked_mul(i128::from(rate.numerator))
            .and_then(|value| value.checked_mul(i128::from(self.rate.denominator)))
            .ok_or_else(arithmetic_overflow)?;
        let denominator = i128::from(self.rate.numerator)
            .checked_mul(i128::from(rate.denominator))
            .ok_or_else(arithmetic_overflow)?;
        if numerator % denominator != 0 {
            return Err(Error::new(
                ErrorKind::InvalidArgument,
                "rational time is not exactly representable at the target rate",
            ));
        }
        let value = i64::try_from(numerator / denominator).map_err(|_| arithmetic_overflow())?;
        Ok(Self::new(value, rate))
    }
}

impl fmt::Display for RationalTime {
    /// Formats the value and normalized rate as `value@numerator/denominator`.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}@{}", self.value, self.rate)
    }
}

impl FromStr for RationalTime {
    type Err = Error;

    /// Parses the canonical `value@numerator/denominator` representation.
    fn from_str(value: &str) -> Result<Self> {
        let (value, rate) = value.split_once('@').ok_or_else(|| {
            Error::new(
                ErrorKind::InvalidArgument,
                "rational time must use value@numerator/denominator syntax",
            )
        })?;
        let value = value.parse::<i64>().map_err(|_| {
            Error::new(
                ErrorKind::InvalidArgument,
                "rational time value must be a signed integer",
            )
        })?;
        Ok(Self::new(value, rate.parse()?))
    }
}

/// A half-open time range with a non-negative duration.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct TimeRange {
    start: RationalTime,
    duration: RationalTime,
}

impl TimeRange {
    /// Creates a time range whose values use one rate.
    ///
    /// # Errors
    ///
    /// Returns [`ErrorKind::InvalidArgument`] when the duration is negative or
    /// uses a different rate from the start.
    pub fn new(start: RationalTime, duration: RationalTime) -> Result<Self> {
        if duration.value() < 0 {
            return Err(Error::new(
                ErrorKind::InvalidArgument,
                "time range duration must not be negative",
            ));
        }
        if start.rate() != duration.rate() {
            return Err(Error::new(
                ErrorKind::InvalidArgument,
                "time range start and duration must use the same rate",
            ));
        }
        Ok(Self { start, duration })
    }

    /// Returns the inclusive start time.
    #[must_use]
    pub const fn start(self) -> RationalTime {
        self.start
    }

    /// Returns the non-negative duration.
    #[must_use]
    pub const fn duration(self) -> RationalTime {
        self.duration
    }

    /// Returns the exclusive end, checking integer overflow.
    ///
    /// # Errors
    ///
    /// Returns [`ErrorKind::InvalidArgument`] when start plus duration exceeds
    /// the range of [`i64`].
    pub fn end_exclusive(self) -> Result<RationalTime> {
        let value = self
            .start
            .value()
            .checked_add(self.duration.value())
            .ok_or_else(arithmetic_overflow)?;
        Ok(RationalTime::new(value, self.start.rate()))
    }
}

impl fmt::Display for TimeRange {
    /// Formats the range as `start+duration` using canonical rational times.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}+{}", self.start, self.duration)
    }
}

impl FromStr for TimeRange {
    type Err = Error;

    /// Parses the canonical `start+duration` representation.
    fn from_str(value: &str) -> Result<Self> {
        let (start, duration) = value.split_once('+').ok_or_else(|| {
            Error::new(
                ErrorKind::InvalidArgument,
                "time range must use start+duration syntax",
            )
        })?;
        Self::new(start.parse()?, duration.parse()?)
    }
}

fn comparison_product(value: i64, denominator: u32, other_numerator: u32) -> Result<i128> {
    i128::from(value)
        .checked_mul(i128::from(denominator))
        .and_then(|product| product.checked_mul(i128::from(other_numerator)))
        .ok_or_else(arithmetic_overflow)
}

fn arithmetic_overflow() -> Error {
    Error::new(
        ErrorKind::InvalidArgument,
        "rational time arithmetic overflow",
    )
}

const fn greatest_common_divisor(mut left: u32, mut right: u32) -> u32 {
    while right != 0 {
        let remainder = left % right;
        left = right;
        right = remainder;
    }
    left
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rate_is_positive_and_normalized() {
        let rate = RationalRate::new(60_000, 2_002).expect("valid rate");

        assert_eq!(rate.numerator(), 30_000);
        assert_eq!(rate.denominator(), 1_001);
        assert!(RationalRate::new(0, 1).is_err());
        assert!(RationalRate::new(24, 0).is_err());
    }

    #[test]
    fn range_requires_one_rate_and_non_negative_duration() {
        let film = RationalRate::new(24, 1).expect("valid rate");
        let video = RationalRate::new(25, 1).expect("valid rate");
        let start = RationalTime::new(100, film);

        assert!(TimeRange::new(start, RationalTime::new(48, film)).is_ok());
        assert!(TimeRange::new(start, RationalTime::new(-1, film)).is_err());
        assert!(TimeRange::new(start, RationalTime::new(50, video)).is_err());
    }

    #[test]
    fn comparison_and_rescaling_are_exact_across_rates() {
        let film = RationalRate::new(24, 1).expect("valid rate");
        let video = RationalRate::new(48, 1).expect("valid rate");
        let one_second_film = RationalTime::new(24, film);
        let one_second_video = RationalTime::new(48, video);

        assert!(one_second_film.equivalent(one_second_video).unwrap());
        assert_eq!(
            one_second_film.rescaled_to(video).unwrap(),
            one_second_video
        );
        assert!(RationalTime::new(1, film).rescaled_to(video).is_ok());
        assert!(RationalTime::new(1, video).rescaled_to(film).is_err());
    }

    #[test]
    fn checked_time_operations_reject_i64_overflow() {
        let rate = RationalRate::new(1, 1).expect("valid rate");
        let faster = RationalRate::new(u32::MAX, 1).expect("valid rate");

        assert!(
            RationalTime::new(i64::MAX, rate)
                .rescaled_to(faster)
                .is_err()
        );
        let range = TimeRange::new(
            RationalTime::new(i64::MAX, rate),
            RationalTime::new(1, rate),
        )
        .expect("valid range");
        assert!(range.end_exclusive().is_err());
    }

    #[test]
    fn canonical_text_round_trips_common_rational_rates() {
        for text in ["24/1", "25/1", "30000/1001", "60000/1001"] {
            let rate: RationalRate = text.parse().expect("parse common rate");
            assert_eq!(rate.to_string(), text);
        }

        let time: RationalTime = "-1001@30000/1001".parse().expect("parse time");
        assert_eq!(time.to_string(), "-1001@30000/1001");
        let range: TimeRange = "-1001@30000/1001+2002@30000/1001"
            .parse()
            .expect("parse range");
        assert_eq!(range.to_string(), "-1001@30000/1001+2002@30000/1001");
    }

    #[test]
    fn canonical_text_rejects_ambiguous_or_invalid_values() {
        assert!("23.976".parse::<RationalRate>().is_err());
        assert!("1@0/1".parse::<RationalTime>().is_err());
        assert!("1@24/1+-1@24/1".parse::<TimeRange>().is_err());
        assert!("1@24/1+1@25/1".parse::<TimeRange>().is_err());
    }
}
