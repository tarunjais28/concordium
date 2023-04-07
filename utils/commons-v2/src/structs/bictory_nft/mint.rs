use super::*;

/// Minting Data.
#[derive(Serialize, SchemaType, Clone, Eq, PartialEq)]
pub struct MintData<T: IsTokenId> {
    /// TokenId to mint
    pub token_id: T,
    /// Address of the creator
    pub owner: Address,
    /// Address of the creator
    pub creator: Address,
    /// Royalty percentage for creator
    pub creator_royalty: u32,
    /// Royalty percentage for minter
    pub minter_royalty: u32,
    /// IPFS content identifier
    pub cid: Vec<u8>,
    /// Copies of NFT
    pub quantity: TokenAmountU64,
}
