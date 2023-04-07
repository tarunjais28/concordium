use super::*;

/// The contract state.
#[contract_state(contract = "BictoryListing")]
#[derive(Serialize, SchemaType)]
pub struct State {
    /// Contract Address of storage
    pub storage_address: ContractAddress,
    /// State of the functions, that require
    pub function_states: Option<FunctionStates>,
    /// The contract address of listing token
    pub leaf_contract_address: Option<ContractAddress>,
}

#[derive(Serialize, SchemaType, Clone)]
pub enum FunctionStates {
    List(NFTDetails),
    UnList(ContractTokenId),
}

#[derive(Debug, SchemaType, Serialize, Clone)]
pub struct NFTDetails {
    pub token_id: ContractTokenId,
    pub owner: AccountAddress,
    pub creator: AccountAddress,
    pub creator_royalty: u32,
    pub minter: AccountAddress,
    pub minter_royalty: u32,
    pub price: Amount,
    pub for_sale: bool,
}

#[derive(SchemaType, Serialize, Eq, PartialEq, PartialOrd, Ord, Debug)]
pub struct Shares {
    pub creator: Amount,
    pub minter: Amount,
    pub owner: Amount,
    pub bictory: Amount,
}
