use super::*;

#[derive(Serialize, SchemaType)]
pub struct NFTData {
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
    pub quantity: TokenAmountU64,
}

#[derive(Serialize, SchemaType)]
pub struct ViewAddressState {
    /// Token Data
    pub owned_data: HashMap<ContractTokenId, NFTData>,
    /// The address which are currently enabled as operators for this address.
    pub operators: Vec<Address>,
}

impl ViewAddressState {
    #[inline(always)]
    pub fn new() -> Self {
        Self {
            owned_data: HashMap::default(),
            operators: Vec::default(),
        }
    }
}
