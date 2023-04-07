use commons_v1::Percentage;
use concordium_std::*;

#[derive(Debug, Clone, SchemaType, Serialize)]
pub struct InitParams {
    pub beneficiary: AccountAddress,
    pub percentage: Percentage,
}

#[derive(Debug, Clone, SchemaType, Serialize)]
pub struct ListingInfo {
    pub price: Amount,
}

#[derive(Debug, Clone, SchemaType, Serialize)]
pub enum ViewInternalValueParams {
    Royalty,
    Beneficiary,
}

#[derive(Debug, Clone, SchemaType, Serialize)]
pub enum InternalValue {
    Royalty(Percentage),
    Beneficiary(AccountAddress),
}
