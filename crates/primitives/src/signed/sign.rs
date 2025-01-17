use core::{
    fmt::{self, Write},
    ops,
};

/// Enum to represent the sign of a 256-bit signed integer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(i8)]
pub enum Sign {
    /// Greater than or equal to zero.
    Positive = 1,
    /// Less than zero.
    Negative = -1,
}

impl ops::Mul<Sign> for Sign {
    type Output = Sign;

    fn mul(self, rhs: Sign) -> Self::Output {
        match (self, rhs) {
            (Self::Positive, Self::Positive) => Self::Positive,
            (Self::Positive, Self::Negative) => Self::Negative,
            (Self::Negative, Self::Positive) => Self::Negative,
            (Self::Negative, Self::Negative) => Self::Positive,
        }
    }
}

impl ops::Neg for Sign {
    type Output = Sign;

    fn neg(self) -> Self::Output {
        match self {
            Self::Positive => Self::Negative,
            Self::Negative => Self::Positive,
        }
    }
}

impl ops::Not for Sign {
    type Output = Sign;

    fn not(self) -> Self::Output {
        match self {
            Self::Positive => Self::Negative,
            Self::Negative => Self::Positive,
        }
    }
}

impl fmt::Display for Sign {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match (self, f.sign_plus()) {
            (Self::Positive, false) => Ok(()),
            _ => f.write_char(self.as_char()),
        }
    }
}

impl Sign {
    /// Equality at compile-time.
    pub const fn const_eq(self, other: Self) -> bool {
        self as i8 == other as i8
    }

    /// Returns whether the sign is positive.
    #[inline(always)]
    pub const fn is_positive(&self) -> bool {
        matches!(self, Self::Positive)
    }

    /// Returns whether the sign is negative.
    #[inline(always)]
    pub const fn is_negative(&self) -> bool {
        matches!(self, Self::Negative)
    }

    /// Returns the sign character.
    #[inline(always)]
    pub const fn as_char(&self) -> char {
        match self {
            Self::Positive => '+',
            Self::Negative => '-',
        }
    }
}
