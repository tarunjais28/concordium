use super::*;

/// Token
#[derive(Debug, Serialize, SchemaType, Hash, PartialEq, Eq, Clone)]
pub struct Token {
    pub contract: ContractAddress,
    pub id: ContractTokenId,
}

/// The contract state.
#[contract_state(contract = "BictorySetForSale")]
#[derive(Serialize, SchemaType)]
pub struct State {
    pub token_details: Vec<TokenInfo>,
}

#[derive(Debug, SchemaType, Serialize, Clone)]
pub struct ForSale {
    pub token: Token,
    pub for_sale: bool,
    pub creator: AccountAddress,
    pub creator_royalty: u32,
    pub minter: AccountAddress,
    pub minter_royalty: u32,
    pub price: Amount,
    pub cid: Vec<u8>,
    pub hash: Vec<u8>,
}

#[derive(SchemaType, Serialize)]
pub struct Sales {
    pub owner: AccountAddress,
    #[concordium(size_length = 1)]
    pub sales: Vec<ForSale>,
}

#[derive(Debug, SchemaType, Serialize, Clone)]
pub struct TokenInfo {
    pub token: Token,
    pub owner: AccountAddress,
    pub for_sale: bool,
}
