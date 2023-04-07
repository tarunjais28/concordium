use super::*;

use core::convert::TryInto;
use core::ops::{Add, AddAssign, Mul};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, SchemaType)]
pub struct Percentage(u64);

impl Percentage {
    pub fn from_micro_percent(micro_percent: u64) -> Self {
        Self(micro_percent)
    }

    pub fn from_percent(percent: u64) -> Self {
        Self(percent * 1_000_000)
    }

    pub fn of_amount(amount: Amount, of: Amount) -> Percentage {
        Percentage(
            (amount.micro_ccd as u128 * 100_000_000)
                .checked_div(of.micro_ccd as u128)
                .and_then(|res| res.try_into().ok())
                .unwrap_or(u64::MAX),
        )
    }
}

impl Add for Percentage {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Percentage(self.0 + rhs.0)
    }
}

impl AddAssign for Percentage {
    fn add_assign(&mut self, rhs: Self) {
        self.0 += rhs.0
    }
}

impl Mul<Amount> for Percentage {
    type Output = Amount;

    fn mul(self, rhs: Amount) -> Self::Output {
        Amount::from_micro_ccd((rhs.micro_ccd as u128 * self.0 as u128 / 100_000_000) as u64)
    }
}
