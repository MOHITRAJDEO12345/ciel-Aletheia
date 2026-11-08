use num_bigint::BigInt;
use num_rational::BigRational;
use num_traits::{Zero, One, ToPrimitive};
use std::fmt;

/// Arbitrary-precision rational number (p/q) preventing IEEE-754 floating point drift.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RationalNumber {
    inner: BigRational,
}

impl RationalNumber {
    pub fn new(num: i64, den: i64) -> Self {
        assert!(den != 0, "denominator cannot be zero");
        Self {
            inner: BigRational::new(BigInt::from(num), BigInt::from(den)),
        }
    }

    pub fn from_bigint(val: BigInt) -> Self {
        Self {
            inner: BigRational::from_integer(val),
        }
    }

    pub fn from_i64(val: i64) -> Self {
        Self::new(val, 1)
    }

    pub fn add(&self, other: &Self) -> Self {
        Self { inner: &self.inner + &other.inner }
    }

    pub fn sub(&self, other: &Self) -> Self {
        Self { inner: &self.inner - &other.inner }
    }

    pub fn mul(&self, other: &Self) -> Self {
        Self { inner: &self.inner * &other.inner }
    }

    pub fn div(&self, other: &Self) -> Result<Self, String> {
        if other.inner.is_zero() {
            return Err("Division by zero in rational arithmetic".to_string());
        }
        Ok(Self { inner: &self.inner / &other.inner })
    }

    pub fn abs(&self) -> Self {
        Self {
            inner: if self.inner < BigRational::zero() {
                -self.inner.clone()
            } else {
                self.inner.clone()
            },
        }
    }

    pub fn to_f64(&self) -> f64 {
        self.inner.to_f64().unwrap_or(f64::NAN)
    }

    pub fn numer(&self) -> &BigInt {
        self.inner.numer()
    }

    pub fn denom(&self) -> &BigInt {
        self.inner.denom()
    }

    pub fn is_integer(&self) -> bool {
        self.inner.is_integer()
    }

    pub fn to_i64(&self) -> Option<i64> {
        if self.inner.is_integer() {
            self.inner.numer().to_i64()
        } else {
            None
        }
    }

    /// Compute exact combination: n! / (k! * (n-k)!)
    pub fn comb(n: u64, k: u64) -> Self {
        if k > n {
            return Self::from_i64(0);
        }
        let k = if k > n - k { n - k } else { k };
        let mut result = BigInt::one();
        for i in 1..=k {
            result = (result * BigInt::from(n - i + 1)) / BigInt::from(i);
        }
        Self::from_bigint(result)
    }
}

impl fmt::Display for RationalNumber {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.inner.is_integer() {
            write!(f, "{}", self.inner.numer())
        } else {
            write!(f, "{}/{}", self.inner.numer(), self.inner.denom())
        }
    }
}

impl fmt::Debug for RationalNumber {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Rational({})", self)
    }
}

impl serde::Serialize for RationalNumber {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> serde::Deserialize<'de> for RationalNumber {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        if let Some(slash_idx) = s.find('/') {
            let n: i64 = s[..slash_idx].parse().map_err(serde::de::Error::custom)?;
            let d: i64 = s[slash_idx + 1..].parse().map_err(serde::de::Error::custom)?;
            Ok(Self::new(n, d))
        } else {
            let n: i64 = s.parse().map_err(serde::de::Error::custom)?;
            Ok(Self::from_i64(n))
        }
    }
}
