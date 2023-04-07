use super::*;

// Functions for creating and updating the contract state.
impl<S: HasStateApi> State<S> {
    /// Creates a new state with no listings.
    pub fn empty(state_builder: &mut StateBuilder<S>) -> Self {
        Self {
            listings: state_builder.new_map(),
        }
    }

    /// Add/update the state with the new listing price.
    pub fn list(
        &mut self,
        token: &Token,
        owner: AccountAddress,
        nft_data: NFTData,
    ) -> ContractResult<()> {
        self.listings.insert(
            token.clone(),
            NFTDetails {
                owner,
                creator: get_account_address(nft_data.creator)?,
                creator_royalty: nft_data.creator_royalty,
                minter: get_account_address(nft_data.minter)?,
                minter_royalty: nft_data.minter_royalty,
                price: nft_data.price,
                quantity: nft_data.quantity,
            },
        );

        Ok(())
    }

    /// Remove a listing and fails with UnknownToken, if token is not listed.
    /// Returns the listing price and owner if successful.
    pub fn unlist(&mut self, token: &Token) -> ContractResult<NFTDetails> {
        let nft_details = *self
            .listings
            .entry(token.clone())
            .and_modify(|details| details.quantity -= 1.into())
            .occupied_or(ContractError::Custom(CustomContractError::UnknownToken))?;

        if nft_details.quantity == 0.into() {
            self.listings.remove(token)
        }

        Ok(nft_details)
    }
}
