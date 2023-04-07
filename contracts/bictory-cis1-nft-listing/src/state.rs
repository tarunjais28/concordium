use commons_v1::{Authority, CustomContractError, Percentage, Royalty, Token};
use concordium_cis1::*;
use concordium_std::*;

#[derive(Serialize, SchemaType)]
pub struct ListingData {
    pub owner: AccountAddress,
    pub price: Amount,
    pub platform_fee: Percentage,
    pub royalties: Vec<Royalty>,
}

/// The contract state.
#[derive(Serial, DeserialWithState)]
#[concordium(state_parameter = "S")]
pub struct State<S: HasStateApi> {
    pub authority: Authority<S>,
    pub royalty: Percentage,
    pub beneficiary: AccountAddress,
    pub listings: StateMap<Token, ListingData, S>,
}

// Functions for creating and updating the contract state.
impl<S: HasStateApi> State<S> {
    /// Creates a new state with no listings.
    pub fn new(
        state_builder: &mut StateBuilder<S>,
        beneficiary: AccountAddress,
        royalty: Percentage,
        origin: AccountAddress,
    ) -> Self {
        State {
            authority: Authority::new(state_builder, Address::Account(origin)),
            royalty,
            beneficiary,
            listings: state_builder.new_map(),
        }
    }

    /// Overwrite the state with the new listing price.
    ///
    /// It is safe to overwrite previous listing. This may happen if token has expired and new token with same ID
    /// was listed after that. It doesn't make sense to check ownership of previous token, because new token with
    /// same ID is already owned by this contract, since list function is always invoken as a callback of CIS-1
    /// transfer function.
    pub fn list(&mut self, contract: ContractAddress, id: TokenIdVec, listing: ListingData) {
        self.listings.insert(Token { contract, id }, listing);
    }

    /// Remove a listing and fails with UnknownToken, if token is not listed.
    /// Returns the listing price and owner if successful.
    pub fn unlist(&mut self, token: &Token) -> ReceiveResult<ListingData> {
        self.listings
            .remove_and_get(token)
            .ok_or_else(|| CustomContractError::TokenNotListedForSale.into())
    }
}
