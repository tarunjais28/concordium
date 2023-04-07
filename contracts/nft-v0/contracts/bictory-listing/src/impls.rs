use super::*;

// Functions for creating and updating the contract state.
impl State {
    /// Creates a new state with no listings.
    pub fn empty() -> Self {
        State {
            listings: Map::default(),
        }
    }

    /// Add/update the state with the new listing price.
    pub fn list(&mut self, listing: ListParams, for_sale: bool) {
        self.listings.insert(
            listing.token,
            NFTDetails {
                owner: listing.owner,
                creator: listing.creator,
                creator_royalty: listing.creator_royalty,
                minter: listing.minter,
                minter_royalty: listing.minter_royalty,
                price: listing.price,
                for_sale,
            },
        );
    }

    /// Remove a listing and fails with UnknownToken, if token is not listed.
    /// Returns the listing price and owner if successful.
    pub fn unlist(&mut self, token: &Token) -> ContractResult<NFTDetails> {
        self.listings
            .remove(token)
            .ok_or_else(|| CustomContractError::UnknownToken.into())
    }
}

impl Shares {
    pub fn adjust_owner_share(&mut self) {
        self.owner -= self.creator + self.minter + self.bictory
    }
}
