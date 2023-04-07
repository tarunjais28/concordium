use super::*;

/// The contract state.
#[derive(Serial, DeserialWithState)]
#[concordium(state_parameter = "S")]
pub struct State<S: HasStateApi> {
    pub listings: StateMap<Token, NFTDetails, S>,
}

#[derive(Serialize, SchemaType, Clone, Copy)]
pub struct NFTDetails {
    pub owner: AccountAddress,
    pub creator: AccountAddress,
    pub creator_royalty: u32,
    pub minter: AccountAddress,
    pub minter_royalty: u32,
    pub price: Amount,
    pub quantity: ContractTokenAmount,
}

#[derive(Debug, SchemaType, Serialize)]
pub struct ListParams {
    pub token: Token,
    pub owner: AccountAddress,
}

#[derive(SchemaType, Serialize)]
pub struct BuyParams {
    pub token: Token,
    pub bictory_royalty: u32,
}

/// Update Price Params.
#[derive(Serialize, SchemaType, Clone, Eq, PartialEq)]
pub struct UpdateListingPrice {
    /// Token to update price
    pub token: Token,
    /// New cost of NFT
    pub price: Amount,
}
