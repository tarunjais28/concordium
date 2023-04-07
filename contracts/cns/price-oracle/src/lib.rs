//! A CNS NFT smart contract implementing the Concordium Token Standard CIS1.
//!
//! # Description
//! An instance of this smart contract can operate on a number of different tokens
//! each identified by a token ID. Tokens are shared across all CNS NFT contracts.
//! A token is then globally identified by any of the CNS NFT contract addresses
//! together with the unique token ID.
//!
//! The contract is initialized with the same tokens that were creater earlier by
//! other CNS NFT contracts and then saved in BictoryStorage contract. Tokens can
//! be minted through a `mint` contract function and burnt through a `burn`
//! contract function. These two operations can only be performed by authorized and
//! compatible CNS contract.

#![cfg_attr(not(feature = "std"), no_std)]

mod contract;
mod events;
mod external;
mod state;
