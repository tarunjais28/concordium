use crate::external::PricingParams;
use commons::{Authority, DomainPrice};
use concordium_std::*;

/// Description of scaling domain name pricing policy.
#[derive(Debug, Serialize, SchemaType)]
pub struct ScalingPricing {
    /// The maximum domain name length to be considered short.
    pub short_max_length: u16,
    /// Short domain name price.
    pub short: DomainPrice,
    /// In-order list of prices for every domain name length starting from [short_max_length].
    pub mid: Vec<DomainPrice>,
    /// Long domain name price.
    pub long: DomainPrice,
}

#[derive(Debug, Serialize, SchemaType)]
pub enum DomainPricing {
    Fixed(DomainPrice),
    Scaling(ScalingPricing),
}

/// The contract state.
#[derive(Serial, DeserialWithState)]
#[concordium(state_parameter = "S")]
pub struct State<S: HasStateApi> {
    /// Addresses authorized to update and maintain the contract.
    pub authority: Authority<S>,
    /// Prices for domains.
    pub domain_prices: DomainPricing,
    /// Prices for subdomains.
    pub subdomain_prices: DomainPricing,
}

impl<S: HasStateApi> State<S> {
    /// Creates a new state with given pricing.
    pub fn new(
        state_builder: &mut StateBuilder<S>,
        params: PricingParams,
        origin: AccountAddress,
    ) -> Self {
        Self {
            authority: Authority::new(state_builder, Address::Account(origin)),
            domain_prices: params.domain_pricing,
            subdomain_prices: params.subdomain_pricing,
        }
    }
}
