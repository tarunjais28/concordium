use concordium_std::*;

use crate::state::DomainPricing;

#[derive(Debug, Serialize, SchemaType)]
pub struct PricingParams {
    pub domain_pricing: DomainPricing,
    pub subdomain_pricing: DomainPricing,
}
