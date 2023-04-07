//! It exposes a function for listing NFTs and a function for buying
//! one of the listed NFTs.
#![cfg_attr(not(feature = "std"), no_std)]
use crate::{events::*, helper::*, structs::*};
use commons::{bictory_nft::view::*, *};
use concordium_cis2::*;
use concordium_std::*;

mod contract;
mod events;
mod helper;
mod impls;
mod structs;
