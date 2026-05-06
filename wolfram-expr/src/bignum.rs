//! Arbitrary-precision number types — gated behind `feature = "bignum"`.
//!
//! WXF carries `BigInteger` and `BigReal` tokens for numbers outside the range/
//! precision of `i64` and `f64`. This module wraps those types so they can become
//! variants of [`ExprKind`][crate::ExprKind].

use num_bigint::BigInt;

/// Arbitrary-precision integer — Wolfram Language `BigInteger`.
///
/// Wraps [`num_bigint::BigInt`]. Available when the `bignum` feature is enabled.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BigInteger(pub BigInt);

impl BigInteger {
    /// Construct from a [`BigInt`].
    pub fn new(value: BigInt) -> Self {
        BigInteger(value)
    }

    /// Parse from a decimal-digit string. Returns `None` on parse failure.
    pub fn parse(digits: &str) -> Option<Self> {
        digits.parse::<BigInt>().ok().map(BigInteger)
    }

    /// Borrow the inner `BigInt`.
    pub fn as_bigint(&self) -> &BigInt {
        &self.0
    }

    /// Consume into the inner `BigInt`.
    pub fn into_bigint(self) -> BigInt {
        self.0
    }

    /// Render as a decimal-digit string.
    pub fn to_decimal_string(&self) -> String {
        self.0.to_str_radix(10)
    }
}

impl From<BigInt> for BigInteger {
    fn from(value: BigInt) -> Self {
        BigInteger(value)
    }
}

impl From<i64> for BigInteger {
    fn from(value: i64) -> Self {
        BigInteger(BigInt::from(value))
    }
}

/// Arbitrary-precision real — Wolfram Language `BigReal`.
///
/// No widely-adopted Rust crate matches Wolfram's BigReal semantics (which carry
/// both a value and a precision). The wire representation in WXF is a digit string
/// like `"3.14159265358979323846`50."` — we preserve it as `digits` (a `String`)
/// for lossless WXF round-trip plus an optional `precision` annotation.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BigReal {
    /// The textual representation, including any precision marker (e.g. `` `50 ``).
    pub digits: String,
}

impl BigReal {
    /// Construct from the WXF wire representation (a digit string).
    pub fn new(digits: impl Into<String>) -> Self {
        BigReal {
            digits: digits.into(),
        }
    }

    /// Borrow the digit string.
    pub fn as_str(&self) -> &str {
        &self.digits
    }

    /// Consume into the digit string.
    pub fn into_string(self) -> String {
        self.digits
    }
}

impl From<String> for BigReal {
    fn from(digits: String) -> Self {
        BigReal { digits }
    }
}

impl From<&str> for BigReal {
    fn from(digits: &str) -> Self {
        BigReal {
            digits: digits.to_owned(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn big_integer_parse_and_render() {
        let huge = "12345678901234567890123456789012345678901234567890";
        let n = BigInteger::parse(huge).unwrap();
        assert_eq!(n.to_decimal_string(), huge);
    }

    #[test]
    fn big_integer_from_i64() {
        let n: BigInteger = 42i64.into();
        assert_eq!(n.to_decimal_string(), "42");
    }

    #[test]
    fn big_integer_negative() {
        let n = BigInteger::parse("-99999999999999999999").unwrap();
        assert_eq!(n.to_decimal_string(), "-99999999999999999999");
    }

    #[test]
    fn big_real_construct() {
        let r = BigReal::new("3.14159265358979323846`50.");
        assert_eq!(r.as_str(), "3.14159265358979323846`50.");
    }
}
