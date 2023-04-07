//! It exposes a function for listing NFTs and a function for buying
//! one of the listed NFTs.
#![cfg_attr(not(feature = "std"), no_std)]
use crate::{calculations::*, events::*, structs::*};
use commons::*;
use concordium_cis1::*;
use concordium_std::*;

mod calculations;
mod contract;
mod events;
mod impls;
mod structs;
