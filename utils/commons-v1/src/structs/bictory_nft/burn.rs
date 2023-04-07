use super::*;

/// Burn Params.
#[derive(Serialize, SchemaType, Clone)]
pub struct BurnParams {
    /// Token Owner
    pub owner: Address,
    /// TokenId to mint
    pub token_id: ContractTokenId,
    /// Copies of NFT
    pub quantity: TokenAmount,
}
