#![no_std]

const YEAR_MILLIS: u64 = 1000 * 60 * 60 * (24 * 365 + 6);

pub mod contract;
pub mod external;
pub mod state;
