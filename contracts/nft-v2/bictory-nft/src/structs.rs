use super::*;

#[derive(Serialize, SchemaType)]
pub enum BurnStep {
    Find(ContractTokenId),
    GetInfo(ContractAddress),
}

#[derive(Serialize, SchemaType)]
pub enum FunctionState {
    Mint(Vec<MintData>),
    Transfer(Vec<Transfer<ContractTokenId>>),
    UpdatePrice(UpdatePriceParameter),
    Burn(BurnStep),
}

/// Minting Data.
#[derive(Serialize, SchemaType)]
pub struct MintData {
    /// NFT token ID
    pub token_id: ContractTokenId,
    /// Address of the creator
    pub creator: Address,
    /// Royalty percentage for creator
    pub creator_royalty: u32,
    /// Address of the minter
    pub minter: Address,
    /// Royalty percentage for minter
    pub minter_royalty: u32,
    /// Cost of NFT
    pub price: Amount,
    /// IPFS content identifier
    pub cid: Bytes,
}

/// Minting Data.
#[derive(Serialize, SchemaType)]
pub struct TransferData {
    /// NFT token ID
    pub token_id: ContractTokenId,
    /// The address owning the tokens being transferred.
    pub from: Address,
    /// The address receiving the tokens being transferred.
    pub to: Address,
}

/// The contract state.
// Note: The specification does not specify how to structure the contract state
// and this could be structured in a more space efficient way depending on the use case.
#[contract_state(contract = "BictoryNFT")]
#[derive(Serialize, SchemaType)]
pub struct State {
    /// Contract Address of storage
    pub storage_address: ContractAddress,
    /// State of the functions, that require
    pub function_state: Option<FunctionState>,
}

/// Update Price Params.
#[derive(Serialize, SchemaType, Clone, Eq, PartialEq, concordium_std::hash::Hash)]
pub struct UpdatePriceParams<T: IsTokenId> {
    /// TokenId to update price
    pub token_id: T,
    /// New cost of NFT
    pub price: Amount,
}
