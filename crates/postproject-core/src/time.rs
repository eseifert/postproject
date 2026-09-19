//! Exact rational time values shared by media structures and adapters.

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
}
