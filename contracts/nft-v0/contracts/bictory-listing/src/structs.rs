use super::*;

/// The contract state.
#[contract_state(contract = "BictoryListing")]
#[derive(Serialize, SchemaType)]
pub struct State {
    #[concordium(size_length = 1)]
    pub listings: Map<Token, NFTDetails>,
}

#[derive(Serialize, SchemaType)]
pub struct NFTDetails {
    pub owner: AccountAddress,
    pub creator: AccountAddress,
    pub creator_royalty: u32,
    pub minter: AccountAddress,
    pub minter_royalty: u32,
    pub price: Amount,
    pub for_sale: bool,
}

#[derive(Debug, SchemaType, Serialize, Clone)]
pub struct ListParams {
    pub token: Token,
    pub owner: AccountAddress,
    pub creator: AccountAddress,
    pub creator_royalty: u32,
    pub minter: AccountAddress,
    pub minter_royalty: u32,
    pub price: Amount,
    pub for_sale: bool,
}

#[derive(SchemaType, Serialize)]
pub struct UnlistParams {
    pub token: Token,
    pub owner: AccountAddress,
}

#[derive(SchemaType, Serialize)]
pub struct BuyParams {
    pub token: Token,
    pub bictory_royalty: u32,
}

#[derive(SchemaType, Serialize, Eq, PartialEq, PartialOrd, Ord, Debug)]
pub struct Shares {
    pub creator: Amount,
    pub minter: Amount,
    pub owner: Amount,
    pub bictory: Amount,
}

/// Update Price Params.
#[derive(Serialize, SchemaType, Clone, Eq, PartialEq)]
pub struct UpdateListingPrice {
    /// Token to update price
    pub token: Token,
    /// New cost of NFT
    pub price: Amount,
}
