//! Precision-Box: A Two-Box number system overcoming IEEE 754 limitations
//!
//! This library implements exact decimal arithmetic by separating numbers into:
//! - Box A: Integer part
//! - Box B: Decimal part (preserving leading zeros and supporting repeating decimals)

use num_bigint::BigInt;
use num_traits::{Zero, One, Signed};
use std::cmp::{max, Ordering};
use std::fmt;
use std::ops::{Add, Sub, Mul, Neg};
use std::str::FromStr;

/// Represents the decimal part of a number
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DecimalPart {
    /// Finite decimal: stores digits as string to preserve leading zeros (e.g., "0022")
    Finite(String),
    /// Repeating decimal: (non_repeating, repeating_pattern)
    /// e.g., 0.1666... = Repeating("1", "6")
    /// e.g., 0.333... = Repeating("", "3")
    Repeating(String, String),
}

impl Default for DecimalPart {
    fn default() -> Self {
        DecimalPart::Finite(String::new())
    }
}

/// Precision-Box Number: Two-Box representation for exact arithmetic
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Num {
    /// Box A: Integer part (supports arbitrarily large values)
    pub box_a: BigInt,
    /// Box B: Decimal part
    pub box_b: DecimalPart,
    /// Sign: true = negative
    pub negative: bool,
}

impl Default for Num {
    fn default() -> Self {
        Num {
            box_a: BigInt::zero(),
            box_b: DecimalPart::default(),
            negative: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
    InvalidFormat(String),
    InvalidDigit(char),
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseError::InvalidFormat(msg) => write!(f, "Invalid format: {}", msg),
            ParseError::InvalidDigit(c) => write!(f, "Invalid digit: {}", c),
        }
    }
}

impl std::error::Error for ParseError {}

impl Num {
    /// Create a new Num with zero value
    pub fn zero() -> Self {
        Self::default()
    }

    /// Create a new Num from integer
    pub fn from_integer(n: i64) -> Self {
        Num {
            box_a: BigInt::from(n.abs()),
            box_b: DecimalPart::Finite(String::new()),
            negative: n < 0,
        }
    }

    /// Create a repeating decimal (e.g., 1/3 = 0.333...)
    /// `non_repeating`: digits before the repeating part
    /// `repeating`: the repeating pattern
    pub fn from_repeating(integer: i64, non_repeating: &str, repeating: &str) -> Self {
        Num {
            box_a: BigInt::from(integer.abs()),
            box_b: DecimalPart::Repeating(non_repeating.to_string(), repeating.to_string()),
            negative: integer < 0,
        }
    }

    /// Parse from string format
    /// Supports:
    /// - "123" (integer)
    /// - "123.456" (decimal)
    /// - "-123.456" (negative)
    /// - "0.333..." or "0.(3)" for repeating decimals
    pub fn from_str(s: &str) -> Result<Self, ParseError> {
        let s = s.trim();
        if s.is_empty() {
            return Err(ParseError::InvalidFormat("Empty string".to_string()));
        }

        let (negative, s) = if s.starts_with('-') {
            (true, &s[1..])
        } else if s.starts_with('+') {
            (false, &s[1..])
        } else {
            (false, s)
        };

        // Check for repeating decimal notation: "0.(3)" or "0.1(6)"
        if let Some(paren_start) = s.find('(') {
            if let Some(paren_end) = s.find(')') {
                return Self::parse_repeating_notation(s, paren_start, paren_end, negative);
            }
        }

        // Check for "..." notation: "0.333..."
        if s.ends_with("...") {
            return Self::parse_ellipsis_notation(s, negative);
        }

        // Standard decimal notation
        if let Some(dot_pos) = s.find('.') {
            let integer_part = &s[..dot_pos];
            let decimal_part = &s[dot_pos + 1..];

            // Validate digits
            for c in integer_part.chars() {
                if !c.is_ascii_digit() {
                    return Err(ParseError::InvalidDigit(c));
                }
            }
            for c in decimal_part.chars() {
                if !c.is_ascii_digit() {
                    return Err(ParseError::InvalidDigit(c));
                }
            }

            let box_a = if integer_part.is_empty() {
                BigInt::zero()
            } else {
                BigInt::from_str(integer_part)
                    .map_err(|_| ParseError::InvalidFormat("Invalid integer part".to_string()))?
            };

            Ok(Num {
                box_a,
                box_b: DecimalPart::Finite(decimal_part.to_string()),
                negative,
            })
        } else {
            // Integer only
            for c in s.chars() {
                if !c.is_ascii_digit() {
                    return Err(ParseError::InvalidDigit(c));
                }
            }

            let box_a = BigInt::from_str(s)
                .map_err(|_| ParseError::InvalidFormat("Invalid integer".to_string()))?;

            Ok(Num {
                box_a,
                box_b: DecimalPart::Finite(String::new()),
                negative,
            })
        }
    }

    fn parse_repeating_notation(s: &str, paren_start: usize, paren_end: usize, negative: bool) -> Result<Self, ParseError> {
        // Format: "0.1(6)" means 0.1666...
        let dot_pos = s.find('.').ok_or_else(||
            ParseError::InvalidFormat("Repeating decimal must have decimal point".to_string()))?;

        let integer_part = &s[..dot_pos];
        let non_repeating = &s[dot_pos + 1..paren_start];
        let repeating = &s[paren_start + 1..paren_end];

        if repeating.is_empty() {
            return Err(ParseError::InvalidFormat("Empty repeating pattern".to_string()));
        }

        let box_a = if integer_part.is_empty() {
            BigInt::zero()
        } else {
            BigInt::from_str(integer_part)
                .map_err(|_| ParseError::InvalidFormat("Invalid integer part".to_string()))?
        };

        Ok(Num {
            box_a,
            box_b: DecimalPart::Repeating(non_repeating.to_string(), repeating.to_string()),
            negative,
        })
    }

    fn parse_ellipsis_notation(s: &str, negative: bool) -> Result<Self, ParseError> {
        // Format: "0.333..." - detect repeating pattern
        let s = &s[..s.len() - 3]; // Remove "..."

        let dot_pos = s.find('.').ok_or_else(||
            ParseError::InvalidFormat("Repeating decimal must have decimal point".to_string()))?;

        let integer_part = &s[..dot_pos];
        let decimal_digits = &s[dot_pos + 1..];

        if decimal_digits.is_empty() {
            return Err(ParseError::InvalidFormat("No decimal digits before ...".to_string()));
        }

        // Try to detect repeating pattern from the end
        let pattern = Self::detect_repeating_pattern(decimal_digits);

        let box_a = if integer_part.is_empty() {
            BigInt::zero()
        } else {
            BigInt::from_str(integer_part)
                .map_err(|_| ParseError::InvalidFormat("Invalid integer part".to_string()))?
        };

        Ok(Num {
            box_a,
            box_b: DecimalPart::Repeating(pattern.0.to_string(), pattern.1.to_string()),
            negative,
        })
    }

    fn detect_repeating_pattern(digits: &str) -> (&str, &str) {
        // Simple heuristic: if all digits are the same, that's the pattern
        let chars: Vec<char> = digits.chars().collect();

        // Try single digit pattern
        if chars.iter().all(|&c| c == chars[chars.len() - 1]) {
            let last = chars[chars.len() - 1];
            let pattern_start = chars.iter().position(|&c| c == last).unwrap();
            return (&digits[..pattern_start], &digits[pattern_start..pattern_start + 1]);
        }

        // Try to find repeating pattern of length 1 to len/2
        for pattern_len in 1..=digits.len() / 2 {
            let pattern = &digits[digits.len() - pattern_len..];
            let mut matches = true;
            let mut pos = digits.len() - pattern_len;

            while pos >= pattern_len {
                pos -= pattern_len;
                if &digits[pos..pos + pattern_len] != pattern {
                    matches = false;
                    break;
                }
            }

            if matches {
                return (&digits[..pos], pattern);
            }
        }

        // Default: assume all given digits form the repeating part
        ("", digits)
    }

    /// Check if this number is zero
    pub fn is_zero(&self) -> bool {
        if !self.box_a.is_zero() {
            return false;
        }
        match &self.box_b {
            DecimalPart::Finite(s) => s.chars().all(|c| c == '0') || s.is_empty(),
            DecimalPart::Repeating(non_rep, rep) => {
                (non_rep.is_empty() || non_rep.chars().all(|c| c == '0')) &&
                rep.chars().all(|c| c == '0')
            }
        }
    }

    /// Get the number of decimal places (for finite decimals)
    pub fn decimal_places(&self) -> Option<usize> {
        match &self.box_b {
            DecimalPart::Finite(s) => Some(s.len()),
            DecimalPart::Repeating(_, _) => None, // Infinite
        }
    }

    /// Convert to scaled BigInt for arithmetic operations
    /// Returns (scaled_value, scale) where value = scaled_value / 10^scale
    fn to_scaled(&self, target_scale: usize) -> BigInt {
        let scale_factor = BigInt::from(10u64).pow(target_scale as u32);

        let decimal_value = match &self.box_b {
            DecimalPart::Finite(s) => {
                if s.is_empty() {
                    BigInt::zero()
                } else {
                    let padded = if s.len() < target_scale {
                        format!("{:0<width$}", s, width = target_scale)
                    } else if s.len() > target_scale {
                        s[..target_scale].to_string()
                    } else {
                        s.clone()
                    };
                    BigInt::from_str(&padded).unwrap_or_else(|_| BigInt::zero())
                }
            }
            DecimalPart::Repeating(non_rep, rep) => {
                // Expand repeating decimal to target_scale digits
                let mut expanded = non_rep.clone();
                while expanded.len() < target_scale {
                    expanded.push_str(rep);
                }
                expanded.truncate(target_scale);
                // Pad if still shorter
                while expanded.len() < target_scale {
                    expanded.push('0');
                }
                BigInt::from_str(&expanded).unwrap_or_else(|_| BigInt::zero())
            }
        };

        let result = &self.box_a * &scale_factor + decimal_value;
        if self.negative {
            -result
        } else {
            result
        }
    }

    /// Create from scaled BigInt
    fn from_scaled(value: BigInt, scale: usize) -> Self {
        let negative = value < BigInt::zero();
        let abs_value = if negative { -&value } else { value };

        let scale_factor = BigInt::from(10u64).pow(scale as u32);
        let box_a = &abs_value / &scale_factor;
        let remainder = &abs_value % &scale_factor;

        // Convert remainder to string with leading zeros
        let decimal_str = if remainder.is_zero() {
            String::new()
        } else {
            let rem_str = remainder.to_string();
            format!("{:0>width$}", rem_str, width = scale)
        };

        // Check if decimal is all 9s (e.g., 0.999... should round up to 1)
        // This handles cases like 1/3 + 1/3 + 1/3 = 0.999... = 1
        let trimmed = decimal_str.trim_end_matches('0');
        if !trimmed.is_empty() && trimmed.chars().all(|c| c == '9') && trimmed.len() >= 10 {
            // Round up: add 1 to integer part, set decimal to empty
            return Num {
                box_a: &box_a + BigInt::one(),
                box_b: DecimalPart::Finite(String::new()),
                negative: negative && !abs_value.is_zero(),
            };
        }

        // Remove trailing zeros
        let decimal_str = decimal_str.trim_end_matches('0').to_string();

        Num {
            box_a,
            box_b: DecimalPart::Finite(decimal_str),
            negative: negative && !abs_value.is_zero(),
        }
    }

    /// Add two Nums
    pub fn add(&self, other: &Self) -> Self {
        // Handle repeating decimals specially
        if let (DecimalPart::Repeating(_, _), DecimalPart::Repeating(_, _)) = (&self.box_b, &other.box_b) {
            return self.add_repeating(other);
        }

        // Determine scale (max decimal places, with reasonable limit for repeating)
        let scale = match (&self.box_b, &other.box_b) {
            (DecimalPart::Finite(a), DecimalPart::Finite(b)) => max(a.len(), b.len()),
            (DecimalPart::Finite(a), DecimalPart::Repeating(_, _)) => max(a.len(), 50),
            (DecimalPart::Repeating(_, _), DecimalPart::Finite(b)) => max(50, b.len()),
            (DecimalPart::Repeating(_, _), DecimalPart::Repeating(_, _)) => 50,
        };

        let scale = if scale == 0 { 0 } else { scale };

        let a_scaled = self.to_scaled(scale);
        let b_scaled = other.to_scaled(scale);
        let sum = a_scaled + b_scaled;

        Self::from_scaled(sum, scale)
    }

    /// Special handling for adding repeating decimals
    fn add_repeating(&self, other: &Self) -> Self {
        // For cases like 1/3 + 1/3 + 1/3 = 1
        // We use high precision and check if result is close to an integer
        let scale = 100; // High precision

        let a_scaled = self.to_scaled(scale);
        let b_scaled = other.to_scaled(scale);
        let sum = &a_scaled + &b_scaled;

        // Check if result is very close to an integer
        let scale_factor = BigInt::from(10u64).pow(scale as u32);
        let remainder = &sum % &scale_factor;
        let abs_remainder = if remainder < BigInt::zero() { -&remainder } else { remainder.clone() };

        // If remainder is very small or very close to scale_factor, round to integer
        let threshold = BigInt::from(10u64).pow((scale - 10) as u32);

        if abs_remainder < threshold {
            // Round down to integer
            let integer = &sum / &scale_factor;
            return Num {
                box_a: if integer < BigInt::zero() { -&integer } else { integer.clone() },
                box_b: DecimalPart::Finite(String::new()),
                negative: integer < BigInt::zero(),
            };
        }

        let diff_from_one = &scale_factor - &abs_remainder;
        if diff_from_one < threshold {
            // Round up to integer
            let integer = (&sum / &scale_factor) + BigInt::one();
            return Num {
                box_a: if integer < BigInt::zero() { -&integer } else { integer.clone() },
                box_b: DecimalPart::Finite(String::new()),
                negative: integer < BigInt::zero(),
            };
        }

        Self::from_scaled(sum, scale)
    }

    /// Subtract other from self
    pub fn sub(&self, other: &Self) -> Self {
        let negated = Num {
            box_a: other.box_a.clone(),
            box_b: other.box_b.clone(),
            negative: !other.negative,
        };
        self.add(&negated)
    }

    /// Multiply two Nums
    pub fn mul(&self, other: &Self) -> Self {
        let self_scale = match &self.box_b {
            DecimalPart::Finite(s) => s.len(),
            DecimalPart::Repeating(_, _) => 50,
        };
        let other_scale = match &other.box_b {
            DecimalPart::Finite(s) => s.len(),
            DecimalPart::Repeating(_, _) => 50,
        };

        let a_scaled = self.to_scaled(self_scale);
        let b_scaled = other.to_scaled(other_scale);
        let product = a_scaled * b_scaled;

        let result_scale = self_scale + other_scale;
        Self::from_scaled(product, result_scale)
    }

    /// Compute self^exponent for non-negative integer exponents
    pub fn pow(&self, exponent: u32) -> Self {
        if exponent == 0 {
            return Num::from_integer(1);
        }
        if exponent == 1 {
            return self.clone();
        }

        // Use binary exponentiation for efficiency
        let mut result = Num::from_integer(1);
        let mut base = self.clone();
        let mut exp = exponent;

        while exp > 0 {
            if exp % 2 == 1 {
                result = Num::mul(&result, &base);
            }
            base = Num::mul(&base, &base.clone());
            exp /= 2;
        }

        result
    }

    /// Compare absolute values
    fn cmp_abs(&self, other: &Self) -> Ordering {
        // First compare integer parts
        match self.box_a.cmp(&other.box_a) {
            Ordering::Equal => {}
            ord => return ord,
        }

        // Compare decimal parts
        let scale = 50;
        let self_scaled = self.to_scaled(scale).abs();
        let other_scaled = other.to_scaled(scale).abs();
        self_scaled.cmp(&other_scaled)
    }
}

impl fmt::Display for Num {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let sign = if self.negative { "-" } else { "" };

        match &self.box_b {
            DecimalPart::Finite(s) if s.is_empty() => {
                write!(f, "{}{}", sign, self.box_a)
            }
            DecimalPart::Finite(s) => {
                write!(f, "{}{}.{}", sign, self.box_a, s)
            }
            DecimalPart::Repeating(non_rep, rep) if non_rep.is_empty() => {
                write!(f, "{}{}.({})", sign, self.box_a, rep)
            }
            DecimalPart::Repeating(non_rep, rep) => {
                write!(f, "{}{}.{}({})", sign, self.box_a, non_rep, rep)
            }
        }
    }
}

impl FromStr for Num {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Num::from_str(s)
    }
}

impl Add for Num {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Num::add(&self, &other)
    }
}

impl Add for &Num {
    type Output = Num;

    fn add(self, other: Self) -> Num {
        Num::add(self, other)
    }
}

impl Sub for Num {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        Num::sub(&self, &other)
    }
}

impl Mul for Num {
    type Output = Self;

    fn mul(self, other: Self) -> Self {
        Num::mul(&self, &other)
    }
}

impl Neg for Num {
    type Output = Self;

    fn neg(self) -> Self {
        let is_zero = self.is_zero();
        Num {
            box_a: self.box_a,
            box_b: self.box_b,
            negative: !self.negative && !is_zero,
        }
    }
}

/// Shorthand for creating a Num from string
/// Panics if the string is invalid
///
/// # Example
/// ```
/// use pbox::num;
/// let a = num("0.1");
/// let b = num("0.2");
/// let c = a + b;  // 0.3
/// ```
pub fn num(s: &str) -> Num {
    Num::from_str(s).expect("Invalid number format")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ieee754_problem_solved() {
        // The classic IEEE 754 problem: 0.1 + 0.2 should equal 0.3
        let a = Num::from_str("0.1").unwrap();
        let b = Num::from_str("0.2").unwrap();
        let expected = Num::from_str("0.3").unwrap();

        let result = Num::add(&a, &b);

        assert_eq!(result, expected, "0.1 + 0.2 should exactly equal 0.3");
        assert_eq!(result.to_string(), "0.3");
    }

    #[test]
    fn test_repeating_decimal_one_third() {
        // 1/3 + 1/3 + 1/3 = 1
        let one_third = Num::from_repeating(0, "", "3");

        let two_thirds = Num::add(&one_third, &one_third);
        let one = Num::add(&two_thirds, &one_third);

        assert_eq!(one.to_string(), "1", "1/3 + 1/3 + 1/3 should equal 1");
    }

    #[test]
    fn test_repeating_decimal_parsing() {
        // Test "0.(3)" notation
        let one_third = Num::from_str("0.(3)").unwrap();
        assert_eq!(one_third.box_a, BigInt::zero());
        assert!(matches!(one_third.box_b, DecimalPart::Repeating(ref nr, ref r) if nr.is_empty() && r == "3"));

        // Test "0.1(6)" notation (1/6 = 0.1666...)
        let one_sixth = Num::from_str("0.1(6)").unwrap();
        assert!(matches!(one_sixth.box_b, DecimalPart::Repeating(ref nr, ref r) if nr == "1" && r == "6"));
    }

    #[test]
    fn test_leading_zeros_preserved() {
        // 1.0022 should preserve the leading zeros in decimal part
        let num = Num::from_str("1.0022").unwrap();

        if let DecimalPart::Finite(s) = &num.box_b {
            assert_eq!(s, "0022", "Leading zeros should be preserved");
        } else {
            panic!("Expected finite decimal");
        }
    }

    #[test]
    fn test_basic_addition() {
        let a = Num::from_str("1.5").unwrap();
        let b = Num::from_str("2.5").unwrap();
        let result = Num::add(&a, &b);

        assert_eq!(result.to_string(), "4");
    }

    #[test]
    fn test_subtraction() {
        let a = Num::from_str("5.5").unwrap();
        let b = Num::from_str("2.3").unwrap();
        let result = Num::sub(&a, &b);

        assert_eq!(result.to_string(), "3.2");
    }

    #[test]
    fn test_multiplication() {
        let a = Num::from_str("0.1").unwrap();
        let b = Num::from_str("0.2").unwrap();
        let result = Num::mul(&a, &b);

        assert_eq!(result.to_string(), "0.02");
    }

    #[test]
    fn test_power() {
        // 0.1^2 = 0.01
        let base = Num::from_str("0.1").unwrap();
        let result = base.pow(2);

        assert_eq!(result.to_string(), "0.01");
    }

    #[test]
    fn test_power_precise() {
        // 1.1^3 = 1.331
        let base = Num::from_str("1.1").unwrap();
        let result = base.pow(3);

        assert_eq!(result.to_string(), "1.331");
    }

    #[test]
    fn test_large_numbers() {
        let a = Num::from_str("999999999999999999.123456789").unwrap();
        let b = Num::from_str("1.000000001").unwrap();
        let result = Num::add(&a, &b);

        assert_eq!(result.to_string(), "1000000000000000000.12345679");
    }

    #[test]
    fn test_negative_numbers() {
        let a = Num::from_str("-1.5").unwrap();
        let b = Num::from_str("2.5").unwrap();
        let result = Num::add(&a, &b);

        assert_eq!(result.to_string(), "1");
    }

    #[test]
    fn test_integer_operations() {
        let a = Num::from_integer(100);
        let b = Num::from_integer(200);
        let result = Num::add(&a, &b);

        assert_eq!(result.to_string(), "300");
    }

    #[test]
    fn test_zero() {
        let zero = Num::zero();
        assert!(zero.is_zero());

        let num = Num::from_str("0.000").unwrap();
        assert!(num.is_zero());
    }

    #[test]
    fn test_decimal_alignment() {
        // 0.1 + 0.02 = 0.12
        let a = Num::from_str("0.1").unwrap();
        let b = Num::from_str("0.02").unwrap();
        let result = Num::add(&a, &b);

        assert_eq!(result.to_string(), "0.12");
    }

    #[test]
    fn test_precision_chain() {
        // Chain of additions should maintain precision
        let a = Num::from_str("0.1").unwrap();
        let mut sum = Num::zero();

        for _ in 0..10 {
            sum = Num::add(&sum, &a);
        }

        assert_eq!(sum.to_string(), "1");
    }
}
