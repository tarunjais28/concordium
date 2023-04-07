use super::*;

/// The state in which an auction can be.
#[derive(Debug, Serialize, SchemaType, Eq, PartialEq, PartialOrd)]
pub enum AuctionState {
    /// The auction is either
    /// - still accepting bids or
    /// - not accepting bids because it's past the auction end, but nobody has
    ///   finalized the auction yet.
    NotSoldYet,
    /// The auction is over and the item has been sold to the indicated address.
    Sold(AccountAddress), // winning account's address
    /// The auction is cancelled
    Canceled,
}

/// The state of the smart contract.
/// This is the state that will be shown when the contract is queried using
/// `concordium-client contract show`.
#[contract_state(contract = "BictoryAuction")]
#[derive(Debug, Serialize, SchemaType, Eq, PartialEq)]
pub struct State {
    /// Contract Address of storage
    pub storage_address: ContractAddress,
    /// Has the item been sold?
    pub auction_state: AuctionState,
    /// The highest bid so far (stored explicitly so that bidders can quickly
    /// see it)
    pub highest_bid: Amount,
    /// The sold token (to be displayed to the auction participants), encoded in
    /// ASCII
    pub token_id: ContractTokenId,
    /// Expiration time of the auction at which bids will be closed (to be
    /// displayed to the auction participants)
    pub expiry: Timestamp,
    /// Keeping track of which account bid how much money
    #[concordium(size_length = 2)]
    pub bids: BTreeMap<AccountAddress, Amount>,
    /// The contract address of auction token
    pub leaf_contract_address: Option<ContractAddress>,
}

/// Type of the parameter to the `init` function.
#[derive(Serialize, SchemaType)]
pub struct InitParameter {
    /// Contract Address of storage
    pub storage_address: ContractAddress,
    /// The token to be sold.
    pub token_id: ContractTokenId,
    /// Time of the auction end in the RFC 3339 format (https://tools.ietf.org/html/rfc3339)
    pub expiry: Timestamp,
}
