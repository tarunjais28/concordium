use super::*;

/// Minting Data.
#[derive(Serial, DeserialWithState, Deletable, Clone, Eq, PartialEq)]
#[concordium(state_parameter = "S")]
pub struct OwnedData<S: HasStateApi> {
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
    pub cid: Vec<u8>,
    /// Copies of NFT
    pub quantity: ContractTokenAmount,
    /// Unused variable
    pub phantom_data: PhantomData<S>,
}

/// The state for each address.
#[derive(Serial, DeserialWithState, Deletable)]
#[concordium(state_parameter = "S")]
pub struct AddressState<S: HasStateApi> {
    /// Token Data
    pub owned_tokens: StateMap<ContractTokenId, OwnedData<S>, S>,
    /// The address which are currently enabled as operators for this address.
    pub operators: StateSet<Address, S>,
}

/// The contract state.
// Note: The specification does not specify how to structure the contract state
// and this could be structured in a more space efficient way depending on the use case.
#[derive(Serial, DeserialWithState)]
#[concordium(state_parameter = "S")]
pub struct State<S: HasStateApi> {
    /// The state for each address.
    pub state: StateMap<Address, AddressState<S>, S>,
    /// All of the token IDs
    pub all_tokens: StateSet<ContractTokenId, S>,
}
