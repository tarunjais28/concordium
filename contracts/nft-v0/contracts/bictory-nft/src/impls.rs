use super::*;

// Functions for creating, updating and querying the contract state.
impl State {
    /// Creates a empty state with no tokens.
    pub fn empty() -> Self {
        State {
            state: Map::default(),
        }
    }

    /// Mint a new token with a given address as the owner and creator
    pub fn mint(
        &mut self,
        owner: Address,
        params: MintData<ContractTokenId>,
        price: Amount,
    ) -> ContractResult<()> {
        let owner_address = self.state.entry(owner).or_default();

        owner_address.token_data.insert(owner, params, price);

        Ok(())
    }

    /// Get the current balance of a given token ID for a given address.
    /// Results in an error if the token ID does not exist in the state.
    /// Since this contract only contains NFTs, the balance will always be
    /// either 1 or 0.
    pub fn balance(
        &self,
        token_id: &ContractTokenId,
        address: &Address,
    ) -> ContractResult<TokenAmount> {
        let balance = self
            .state
            .get(address)
            .map(|address_state| {
                if address_state.token_data.owned_tokens.contains_key(token_id) {
                    1
                } else {
                    0
                }
            })
            .unwrap_or(0);
        Ok(balance)
    }

    /// Check if a given address is an operator of a given owner address.
    pub fn is_operator(&self, address: &Address, owner: &Address) -> bool {
        self.state
            .get(owner)
            .map(|address_state| address_state.operators.contains(address))
            .unwrap_or(false)
    }

    /// Update the state with a transfer of some token.
    /// Results in an error if the token ID does not exist in the state or if
    /// the from address have insufficient tokens to do the transfer.
    pub fn transfer(&mut self, transfer: &Transfer<ContractTokenId>) -> ContractResult<()> {
        // A zero transfer does not modify the state.
        if transfer.amount == 0 {
            return Ok(());
        }

        // Deriving `for_sale` field from data field of transfer.
        let for_sale = if let Some(byte) = transfer.data.as_ref().get(0) {
            *byte != 0
        } else {
            false
        };

        // Since this contract only contains NFTs, no one will have an amount greater
        // than 1. And since the amount cannot be the zero at this point, the
        // address must have insufficient funds for any amount other than 1.
        ensure_eq!(transfer.amount, 1, ContractError::InsufficientFunds);

        if let Some(from_address_state) = self.state.get_mut(&transfer.from) {
            // Removing fields from `from` account and extract minting data
            let owned_data = from_address_state.token_data.remove(&transfer.token_id)?;

            // Add the token to the new owner.
            let to_address_state = self.state.entry(transfer.to.address()).or_default();
            to_address_state
                .token_data
                .transfer(transfer.token_id.clone(), owned_data, for_sale);

            Ok(())
        } else {
            Err(ContractError::Custom(CustomContractError::AddressNotFound))
        }
    }

    /// Update the state adding a new operator for a given address.
    /// Succeeds even if the `operator` is already an operator for the
    /// `address`.
    pub fn add_operator(&mut self, owner: &Address, operator: &Address) {
        let owner_address_state = self.state.entry(*owner).or_default();
        owner_address_state.operators.insert(*operator);
    }

    /// Update the state removing an operator for a given address.
    /// Succeeds even if the `operator` is _not_ an operator for the `address`.
    pub fn remove_operator(&mut self, owner: &Address, operator: &Address) {
        self.state
            .get_mut(owner)
            .map(|address_state| address_state.operators.remove(operator));
    }

    /// Burning of NFT.
    /// Results in an error if the
    /// - token ID does not exist in the state
    /// - owner's address not found
    pub fn burn(
        &mut self,
        owner: &Address,
        token_id: ContractTokenId,
    ) -> ContractResult<BurnEvent<ContractTokenId>> {
        // Extracting owner account state associated with given owner address
        let addr_state = self
            .state
            .get_mut(owner)
            .ok_or(ContractError::Custom(CustomContractError::AddressNotFound))?;

        // Extracting token_details
        let _ = addr_state
            .token_data
            .owned_tokens
            .remove(&token_id)
            .ok_or(ContractError::InvalidTokenId)?;

        Ok(BurnEvent {
            token_id,
            amount: 1,
            owner: *owner,
        })
    }

    /// Updating price of NFT.
    /// Results in an error if the
    /// - token ID does not exist in the state
    /// - owner's address not found
    pub fn update_price(
        &mut self,
        owner: &Address,
        params: UpdatePriceParameter,
    ) -> ContractResult<UpdatePriceEvent<ContractTokenId>> {
        // Extracting owner account state associated with given owner address
        let addr_state = self
            .state
            .get_mut(owner)
            .ok_or(ContractError::Custom(CustomContractError::AddressNotFound))?;

        // Extracting token_details
        let token_data = addr_state
            .token_data
            .owned_tokens
            .get_mut(&params.token_id)
            .ok_or(ContractError::InvalidTokenId)?;

        // Updating token price
        let from = token_data.price.micro_ccd;
        token_data.price = params.price;

        Ok(UpdatePriceEvent {
            token_id: params.token_id,
            owner: *owner,
            from,
            to: token_data.price.micro_ccd,
        })
    }

    /// Remove empty state
    pub fn clear_empty_state(&mut self, owner: &Address) {
        if let Some(addr_state) = self.state.get_mut(owner) {
            if addr_state.token_data.owned_tokens.is_empty() {
                self.state.remove(owner);
            }
        };
    }
}

impl TokenData {
    fn remove(&mut self, token_id: &ContractTokenId) -> ContractResult<OwnedData> {
        // Find and remove the token from the owner, if nothing is removed, we know the
        // address did not own the token.

        if let Some((_, owned_data)) = self.owned_tokens.remove_entry(token_id) {
            Ok(owned_data)
        } else {
            Err(ContractError::InvalidTokenId)
        }
    }

    #[inline(always)]
    fn insert(&mut self, owner: Address, mint_data: MintData<ContractTokenId>, price: Amount) {
        // During minting making for_sale as `false` and it will be
        // automatically `true` during minting.
        let for_sale = false;

        self.owned_tokens.insert(
            mint_data.token_id,
            OwnedData {
                creator: mint_data.creator,
                price,
                for_sale,
                cid: mint_data.cid,
                creator_royalty: mint_data.creator_royalty,
                minter: owner,
                minter_royalty: mint_data.minter_royalty,
            },
        );
    }

    #[inline(always)]
    fn transfer(&mut self, token_id: ContractTokenId, owner_data: OwnedData, for_sale: bool) {
        self.owned_tokens.insert(
            token_id,
            OwnedData {
                for_sale,
                ..owner_data
            },
        );
    }
}
