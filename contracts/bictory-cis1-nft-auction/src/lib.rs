//! It exposes a function for listing NFTs and a function for buying
//! one of the listed NFTs.
#![cfg_attr(not(feature = "std"), no_std)]

mod contract;
mod events;
mod external;
mod nft;
mod state;
