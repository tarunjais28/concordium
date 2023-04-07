use super::*;

/// Minting Data.
#[derive(Serialize, SchemaType, Clone, Eq, PartialEq, concordium_std::hash::Hash)]
pub struct OwnedData {
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
    /// Flag to decide whether the NFT is put for sale or not
    pub for_sale: bool,
    /// IPFS content identifier
    pub cid: Vec<u8>,
}

/// The state for each address.
#[derive(Serialize, SchemaType, Default)]
pub struct TokenData {
    /// HashMap to track minting data for the tokens.
    pub owned_tokens: Map<ContractTokenId, OwnedData>,
}

/// The state for each address.
#[derive(Serialize, SchemaType, Default)]
pub struct AddressState {
    /// Token Data
    pub token_data: TokenData,
    /// The address which are currently enabled as operators for this address.
    #[concordium(size_length = 1)]
    pub operators: Set<Address>,
}

/// The contract state.
// Note: The specification does not specify how to structure the contract state
// and this could be structured in a more space efficient way depending on the use case.
#[contract_state(contract = "BictoryNFT")]
#[derive(Serialize, SchemaType)]
pub struct State {
    /// The state for each address.
    pub state: Map<Address, AddressState>,
}

/// The parameter for the contract function `mint` which mints a number of
/// tokens to a given address.
#[derive(Serialize, SchemaType)]
pub struct MintParams<T: IsTokenId + Eq + PartialEq + concordium_std::hash::Hash> {
    /// Minting Data
    pub mint_data: Set<MintData<T>>,
}

/// Minting Data.
#[derive(Serialize, SchemaType, Clone, Eq, PartialEq, concordium_std::hash::Hash)]
pub struct MintData<T: IsTokenId> {
    /// TokenId to mint
    pub token_id: T,
    /// Address of the creator
    pub creator: Address,
    /// Royalty percentage for creator
    pub creator_royalty: u32,
    /// Royalty percentage for minter
    pub minter_royalty: u32,
    /// IPFS content identifier
    pub cid: Vec<u8>,
    /// Royalty percentage for minter
    pub bictory_royalty: u32,
}

#[derive(SchemaType, Serialize, Eq, PartialEq, PartialOrd, Ord, Debug)]
pub struct Shares {
    pub creator: Amount,
    pub bictory: Amount,
}
