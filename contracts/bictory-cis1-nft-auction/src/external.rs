use commons_v1::Percentage;
use concordium_std::*;

#[derive(Debug, Clone, SchemaType, Serialize)]
pub struct InitParams {
    pub beneficiary: AccountAddress,
    pub royalty: Percentage,
}

/// Bid increment policy.
#[derive(Debug, Clone, Copy, Serialize, SchemaType)]
pub enum BidIncrement {
    /// Bid must be incremented by fixed amount.
    Flat(Amount),
    /// Bid must be incremented by percentage of current bid.
    Percentage(Percentage),
}

/// Auction finalization policy.
#[derive(Debug, Clone, SchemaType, Serialize)]
pub enum Finalization {
    /// Auction lasts exact duration.
    Duration(Duration),
    /// Auction ends when given duration has passed betweed bids.
    BidTimeout(Duration),
}

#[derive(Debug, Clone, SchemaType, Serialize)]
pub struct LotInfo {
    /// Auction start time. Immediate by default.
    pub start: Option<Timestamp>,
    /// Auction finalization policy.
    pub finalization: Finalization,
    /// Smallest allowed bid.
    pub reserve: Amount,
    /// Bid increment policy.
    pub increment: BidIncrement,
    /// Buyout price. Buyout is not allowed by default.
    pub buyout: Option<Amount>,
}

#[derive(Debug, Clone, SchemaType, Serialize)]
pub enum InternalValue {
    Royalty(Percentage),
    Beneficiary(AccountAddress),
}

#[derive(Debug, Clone, SchemaType, Serialize)]
pub enum ViewInternalValueParams {
    Royalty,
    Beneficiary,
}
