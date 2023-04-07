use super::*;

/// Update Price Params.
#[derive(Serialize, SchemaType, Clone, Eq, PartialEq)]
pub struct UpdatePriceParams<T: IsTokenId> {
    /// TokenId to update price
    pub token_id: T,
    /// New cost of NFT
    pub price: Amount,
}
