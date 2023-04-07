//! A NFT smart contract example using the Concordium Token Standard CIS1.
//!
//! # Description
//! An instance of this smart contract can contain a number of different token
//! each identified by a token ID. A token is then globally identified by the
//! contract address together with the token ID.
//!
//! In this example the contract is initialized with no tokens, and tokens can
//! be minted through a `mint` contract function, which will only succeed for
//! the contract owner. No functionality to burn token is defined in this
//! example.
//!
//! Note: The word 'address' refers to either an account address or a
//! contract address.
//!
//! As follows from the CIS1 specification, the contract has a `transfer`
//! function for transferring an amount of a specific token type from one
//! address to another address. An address can enable and disable one or more
//! addresses as operators. An operator of some address is allowed to transfer
//! any tokens owned by this address.

#![cfg_attr(not(feature = "std"), no_std)]
use crate::{calculations::*, constants::*, events::*, structs::*, types::*};
use commons::*;
use concordium_cis1::*;
use concordium_std::{
    collections::{HashMap as Map, HashSet as Set},
    *,
};

mod calculations;
mod constants;
mod contract;
mod events;
mod impls;
mod structs;
mod types;

/// Build a string from TOKEN_METADATA_BASE_URL appended with the token ID
/// encoded as hex.
fn build_token_metadata_url(token_id: &ContractTokenId) -> String {
    let mut token_metadata_url = String::from(TOKEN_METADATA_BASE_URL);
    push_token_id(&mut token_metadata_url, token_id);
    token_metadata_url.push('/');
    token_metadata_url
}

fn push_token_id(string: &mut String, token_id: &ContractTokenId) {
    for byte in &token_id.0 {
        string.push(bits_to_hex_char(byte >> 4));
        string.push(bits_to_hex_char(byte & 0xF));
    }
}

fn bits_to_hex_char(bits: u8) -> char {
    match bits & 0xF {
        0x0..=0x9 => (bits + b'0') as char,
        0xA..=0xF => (bits - 10 + b'A') as char,
        _ => unreachable!(),
    }
}

#[concordium_cfg_test]
mod tests {
    use super::*;

    #[concordium_test]
    fn token_id_formatting() {
        for x in 0x00u8..0xFF {
            let mut counter = x;
            let token_bytes = core::iter::repeat_with(|| {
                let res = counter;
                counter = counter.wrapping_add(0x55);
                res
            })
            .take(x as usize % 10 + 1)
            .collect();

            let token_id: ContractTokenId = TokenIdVec(token_bytes);

            let mut token_id_string = String::new();
            push_token_id(&mut token_id_string, &token_id);
            claim_eq!(token_id_string, token_id.to_string());
        }
    }
}
