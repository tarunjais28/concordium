use super::*;

impl<S: HasStateApi> AddressState<S> {
    fn empty(state_builder: &mut StateBuilder<S>) -> Self {
        Self {
            operators: state_builder.new_set(),
            owned_tokens: state_builder.new_map(),
        }
    }

    #[inline(always)]
    fn insert(&mut self, owner: Address, mint_data: MintData<ContractTokenId>, price: Amount) {
        self.owned_tokens.insert(
            mint_data.token_id,
            OwnedData {
                creator: mint_data.creator,
                price,
                cid: mint_data.cid.to_vec(),
                creator_royalty: mint_data.creator_royalty,
                minter: owner,
                minter_royalty: mint_data.minter_royalty,
                quantity: mint_data.quantity,
                phantom_data: PhantomData,
            },
        );
    }
}

// Functions for creating, updating and querying the contract state.
impl<S: HasStateApi> State<S> {
    /// Creates a empty state with no tokens.
    pub fn empty(state_builder: &mut StateBuilder<S>) -> Self {
        State {
            state: state_builder.new_map(),
            all_tokens: state_builder.new_set(),
        }
    }

    /// Mint a new token with a given address as the owner and creator
    pub fn mint(
        &mut self,
        params: MintData<ContractTokenId>,
        price: Amount,
        state_builder: &mut StateBuilder<S>,
    ) -> ContractResult<()> {
        ensure!(
            self.all_tokens.insert(params.token_id.clone()),
            CustomContractError::TokenIdAlreadyExists.into()
        );

        let mut owner_address = self
            .state
            .entry(params.owner)
            .or_insert_with(|| AddressState::empty(state_builder));

        owner_address.insert(params.owner, params, price);
        Ok(())
    }

    /// Check that the token ID currently exists in this contract.
    #[inline(always)]
    fn contains_token(&self, token_id: &ContractTokenId) -> bool {
        self.all_tokens.contains(token_id)
    }

    /// Get the current balance of a given token ID for a given address.
    /// Results in an error if the token ID does not exist in the state.
    pub fn balance(
        &self,
        token_id: &ContractTokenId,
        address: &Address,
    ) -> ContractResult<ContractTokenAmount> {
        ensure!(self.contains_token(token_id), ContractError::InvalidTokenId);

        Ok(self.state.get(address).map_or(0.into(), |address_state| {
            address_state
                .owned_tokens
                .get(token_id)
                .map_or(0.into(), |owned_data| owned_data.quantity)
        }))
    }

    /// Check if a given address is an operator of a given owner address.
    pub fn is_operator(&self, owner: &Address, address: &Address) -> bool {
        self.state
            .get(owner)
            .map(|address_state| address_state.operators.contains(address))
            .unwrap_or(false)
    }

    /// Update the state with a transfer of some token.
    /// Results in an error if the token ID does not exist in the state or if
    /// the from address have insufficient tokens to do the transfer.
    pub fn transfer(
        &mut self,
        transfer: &Transfer<ContractTokenId, ContractTokenAmount>,
        state_builder: &mut StateBuilder<S>,
    ) -> ContractResult<()> {
        ensure!(
            self.contains_token(&transfer.token_id),
            ContractError::InvalidTokenId
        );

        // A zero transfer does not modify the state.
        if transfer.amount == 0.into() {
            return Ok(());
        }

        let mut owned_data = {
            let balance = Self::balance(self, &transfer.token_id, &transfer.from)?;

            let mut from_address_state = self
                .state
                .entry(transfer.from)
                .occupied_or(CustomContractError::AddressNotFound)?;

            match balance.cmp(&transfer.amount) {
                core::cmp::Ordering::Equal => {
                    let mut owned_data = from_address_state
                        .owned_tokens
                        .remove_and_get(&transfer.token_id)
                        .ok_or(ContractError::InvalidTokenId)?;
                    owned_data.quantity -= transfer.amount;
                    owned_data
                }
                core::cmp::Ordering::Greater => {
                    let mut owned_data = from_address_state
                        .owned_tokens
                        .entry(transfer.token_id.clone())
                        .occupied_or(ContractError::InvalidTokenId)?;
                    owned_data.quantity -= transfer.amount;
                    owned_data.copy()
                }
                core::cmp::Ordering::Less => {
                    return Err(ContractError::InsufficientFunds);
                }
            }
        };

        let mut to_address_state = self
            .state
            .entry(transfer.to.address())
            .or_insert_with(|| AddressState::empty(state_builder));

        to_address_state
            .owned_tokens
            .entry(transfer.token_id.clone())
            .and_modify(|data| data.quantity += transfer.amount)
            .or_insert_with(|| {
                owned_data.quantity = transfer.amount;
                owned_data
            });

        Ok(())
    }

    /// Update the state adding a new operator for a given address.
    /// Succeeds even if the `operator` is already an operator for the
    /// `address`.
    pub fn add_operator(
        &mut self,
        owner: &Address,
        operator: &Address,
        state_builder: &mut StateBuilder<S>,
    ) {
        let mut owner_address_state = self
            .state
            .entry(*owner)
            .or_insert_with(|| AddressState::empty(state_builder));
        owner_address_state.operators.insert(*operator);
    }

    /// Update the state removing an operator for a given address.
    /// Succeeds even if the `operator` is _not_ an operator for the `address`.
    pub fn remove_operator(&mut self, owner: &Address, operator: &Address) {
        self.state
            .get_mut(owner)
            .map(|mut address_state| address_state.operators.remove(operator));
    }

    /// Burning of NFT.
    /// Results in an error if the
    /// - token ID does not exist in the state
    /// - owner's address not found
    pub fn burn(
        &mut self,
        owner: &Address,
        params: BurnParams,
    ) -> ContractResult<BurnEvent<ContractTokenId, ContractTokenAmount>> {
        // Extracting owner account state associated with given owner address
        let mut addr_state = self
            .state
            .get_mut(owner)
            .ok_or(ContractError::Custom(CustomContractError::AddressNotFound))?;

        let balance = Self::balance(self, &params.token_id, owner)?;

        match balance.cmp(&params.quantity) {
            core::cmp::Ordering::Equal => {
                addr_state.owned_tokens.remove(&params.token_id);
            }
            core::cmp::Ordering::Greater => addr_state
                .owned_tokens
                .get_mut(&params.token_id)
                .map(|mut owned_data| owned_data.quantity -= params.quantity)
                .ok_or(ContractError::InvalidTokenId)?,
            core::cmp::Ordering::Less => {
                return Err(ContractError::InsufficientFunds);
            }
        }

        Ok(BurnEvent {
            token_id: params.token_id,
            amount: params.quantity,
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
        let mut token_data = addr_state
            .owned_tokens
            .get_mut(&params.token_id)
            .ok_or(ContractError::InvalidTokenId)?;

        // Updating token price
        let from = token_data.price;
        token_data.price = params.price;

        Ok(UpdatePriceEvent {
            token_id: params.token_id,
            owner: *owner,
            from,
            to: token_data.price,
        })
    }
}

impl<S: HasStateApi> OwnedData<S> {
    pub fn as_nft_data(&self) -> NFTData {
        NFTData {
            creator: self.creator,
            creator_royalty: self.creator_royalty,
            minter: self.minter,
            minter_royalty: self.minter_royalty,
            price: self.price,
            cid: self.cid.clone(),
            quantity: self.quantity,
        }
    }

    pub fn copy(&self) -> Self {
        Self {
            creator: self.creator,
            creator_royalty: self.creator_royalty,
            minter: self.minter,
            minter_royalty: self.minter_royalty,
            price: self.price,
            cid: self.cid.clone(),
            quantity: self.quantity,
            phantom_data: PhantomData,
        }
    }
}
