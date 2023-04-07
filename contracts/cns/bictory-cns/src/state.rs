use commons_v1::Authority;
use concordium_std::*;

#[derive(Debug, Serial, DeserialWithState)]
#[concordium(state_parameter = "S")]
pub struct State<S: HasStateApi> {
    /// Contract maintainance rights.
    pub authority: Authority<S>,
    /// Registry BictoryStorage contract address. Responsible for keeping domain data.
    pub registry: ContractAddress,
    /// BictoryCnsNft contract address. Responsible for token ownership.
    pub nft: ContractAddress,
    /// BictoryCnsPriceOracle contract address. Keeps the updated CNS pricing info.
    pub price_oracle: ContractAddress,
    /// account address. Will receive payments from new subscriptions.
    pub beneficiary: AccountAddress,
    /// Maximum subscription year count from slot time.
    pub subscription_year_limit: u8,
}

impl<S: HasStateApi> State<S> {
    pub fn new(
        state_builder: &mut StateBuilder<S>,
        origin: AccountAddress,
        registry: ContractAddress,
        nft: ContractAddress,
        price_oracle: ContractAddress,
        subscription_year_limit: u8,
    ) -> Self {
        Self {
            authority: Authority::new(state_builder, Address::Account(origin)),
            registry,
            nft,
            price_oracle,
            beneficiary: origin,
            subscription_year_limit,
        }
    }
}
