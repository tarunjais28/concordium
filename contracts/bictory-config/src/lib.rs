//! It exposes a function for listing NFTs and a function for buying
//! one of the listed NFTs.
#![cfg_attr(not(feature = "std"), no_std)]
use crate::{events::*, structs::*};
use commons::*;
use concordium_std::*;
use marker::PhantomData;

mod contract;
mod events;
mod impls;
mod structs;
