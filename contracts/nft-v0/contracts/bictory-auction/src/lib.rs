//! # Implementation of an auction smart contract
//!
//! To bid, participants send CCD using the bid function.
//! The participant with the highest bid wins the auction.
//! Bids are to be placed before the auction end. After that, bids are refused.
//! Only bids that exceed the highest bid are accepted.
//! Bids are placed incrementally, i.e., an account's bid is considered
//! to be the **sum** of all bids.
//!
//! Example: if Alice first bid 1 CCD and then bid 2 CCD, her total
//! bid is 3 CCD. The bidding will only go through if 3 CCD is higher than
//! the currently highest bid.
//!
//! After the auction end, any account can finalize the auction.
//! The auction can be finalized only once.
//! When the auction is finalized, every participant except the
//! winner gets their money back.
#![cfg_attr(not(feature = "std"), no_std)]
use crate::{events::*, structs::*};
use commons::*;
use concordium_cis1::*;
use concordium_std::{collections::BTreeMap, *};
use core::fmt::Debug;

mod contract;
mod events;
mod structs;

/// A helper function to create a state for a new auction.
fn fresh_state(itm: Token, exp: Timestamp) -> State {
    State {
        auction_state: AuctionState::NotSoldYet,
        highest_bid: Amount::zero(),
        item: itm,
        expiry: exp,
        bids: BTreeMap::new(),
        is_authorised: false,
    }
}
