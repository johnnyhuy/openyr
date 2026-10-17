//! Fixed-point number for sim maths: i32 with 8 fractional bits
//! (256 sub-units per unit = one lepton when the unit is a cell).
//! No floats anywhere; arithmetic widens to i64 where it could overflow.

use std::ops::{Add, AddAssign, Mul, Neg, Sub};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Fix(i32);

impl Fix {
    pub const ZERO: Fix = Fix(0);
    pub const ONE: Fix = Fix(256);

    pub const fn from_int(v: i32) -> Fix {
        Fix(v << 8)
    }

    /// Raw leptons (the scaled backing value).
    pub const fn raw(self) -> i32 {
        self.0
    }

    pub const fn from_raw(raw: i32) -> Fix {
        Fix(raw)
    }

    /// Whole units, truncating toward negative infinity (cell lookup).
    pub fn to_int_floor(self) -> i32 {
        self.0 >> 8
    }

    pub fn clamp(self, lo: Fix, hi: Fix) -> Fix {
        if self < lo {
            lo
        } else if self > hi {
            hi
        } else {
            self
        }
    }

    pub fn abs(self) -> Fix {
        Fix(self.0.abs())
    }
}

impl Add for Fix {
    type Output = Fix;
    fn add(self, rhs: Fix) -> Fix {
        Fix(self.0 + rhs.0)
    }
}

impl AddAssign for Fix {
    fn add_assign(&mut self, rhs: Fix) {
        self.0 += rhs.0;
    }
}

impl Sub for Fix {
    type Output = Fix;
    fn sub(self, rhs: Fix) -> Fix {
        Fix(self.0 - rhs.0)
    }
}

impl Neg for Fix {
    type Output = Fix;
    fn neg(self) -> Fix {
        Fix(-self.0)
    }
}

impl Mul for Fix {
    type Output = Fix;
    fn mul(self, rhs: Fix) -> Fix {
        Fix(((i64::from(self.0) * i64::from(rhs.0)) >> 8) as i32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixed_point_arithmetic_is_exact() {
        let a = Fix::from_raw(384); // 1.5
        let b = Fix::from_raw(128); // 0.5
        assert_eq!(a + b, Fix::from_int(2));
        assert_eq!(a * b, Fix::from_raw(192)); // 0.75
        assert_eq!(Fix::from_int(3).to_int_floor(), 3);
        assert_eq!(Fix::from_raw(-1).to_int_floor(), -1);
        assert_eq!(
            (Fix::from_int(10) - Fix::from_int(25)).abs(),
            Fix::from_int(15)
        );
    }
}
