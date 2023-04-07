use super::*;

/// The state in which an auction can be.
#[derive(Debug, Serialize, SchemaType, Eq, PartialEq, PartialOrd, Clone)]
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
#[derive(Serial, DeserialWithState)]
#[concordium(state_parameter = "S")]
pub struct State<S: HasStateApi> {
    /// The part of the state that can be viewed
    pub viewable_state: ViewableState,
    /// Keeping track of which account bid how much money
    pub bids: StateMap<AccountAddress, Amount, S>,
}

/// The part of the state to be viewed using `concordium-client contract invoke`
#[derive(Debug, Serialize, SchemaType, Clone)]
pub struct ViewableState {
    /// Has the item been sold?
    pub auction_state: AuctionState,
    /// The highest bid so far (stored explicitly so that bidders can quickly
    /// see it)
    pub highest_bid: Amount,
    /// The sold item (to be displayed to the auction participants), encoded in
    /// ASCII
    pub item: Token,
    /// Expiration time of the auction at which bids will be closed (to be
    /// displayed to the auction
    pub expiry: Timestamp,
    /// Flag to check wheather auction contract is authorized to perform transaction
    /// on NFT contract or not
    pub is_authorised: bool,
}

/// Type of the parameter to the `init` function.
#[derive(Serialize, SchemaType)]
pub struct InitParameter {
    /// The item to be sold.
    pub item: Token,
    /// Time of the auction end in the RFC 3339 format (https://tools.ietf.org/html/rfc3339)
    pub expiry: Timestamp,
}
