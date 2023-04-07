use super::*;

// Functions for creating, updating and querying the contract state.
impl State {
    /// Creates a empty state with no tokens.
    pub fn new(storage_address: ContractAddress) -> Self {
        State {
            storage_address,
            function_state: None,
        }
    }

    // /// Check that the token ID currently exists in this contract.
    // #[inline(always)]
    // pub fn contains_token(&self, token_id: &ContractTokenId) -> bool {
    //     self.all_tokens.iter().any(|id| id == token_id)
    // }

    // /// Get the current balance of a given token ID for a given address.
    // /// Results in an error if the token ID does not exist in the state.
    // /// Since this contract only contains NFTs, the balance will always be
    // /// either 1 or 0.
    // pub fn balance(
    //     &self,
    //     token_id: &ContractTokenId,
    //     address: &Address,
    // ) -> ContractResult<TokenAmount> {
    //     ensure!(self.contains_token(token_id), ContractError::InvalidTokenId);
    //     let balance = self
    //         .state
    //         .get(address)
    //         .map(|address_state| {
    //             if address_state.token_data.owned_tokens.contains_key(token_id) {
    //                 1
    //             } else {
    //                 0
    //             }
    //         })
    //         .unwrap_or(0);
    //     Ok(balance)
    // }

    // /// Check if a given address is an operator of a given owner address.
    // pub fn is_operator(&self, address: &Address, owner: &Address) -> bool {
    //     self.state
    //         .get(owner)
    //         .map(|address_state| address_state.operators.contains(address))
    //         .unwrap_or(false)
    // }

    // /// Update the state adding a new operator for a given address.
    // /// Succeeds even if the `operator` is already an operator for the
    // /// `address`.
    // pub fn add_operator(&mut self, owner: &Address, operator: &Address) {
    //     let owner_address_state = self.state.entry(*owner).or_default();
    //     owner_address_state.operators.insert(*operator);
    // }

    // /// Update the state removing an operator for a given address.
    // /// Succeeds even if the `operator` is _not_ an operator for the `address`.
    // pub fn remove_operator(&mut self, owner: &Address, operator: &Address) {
    //     self.state
    //         .get_mut(owner)
    //         .map(|address_state| address_state.operators.remove(operator));
    // }
}
