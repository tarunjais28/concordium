use super::*;

// Functions for creating and updating the contract state.
impl State {
    /// Creates a new state with no token details.
    pub fn empty() -> Self {
        State {
            token_details: Vec::default(),
        }
    }

    /// Creates a new state with no token details.
    pub fn update_state(
        &mut self,
        owner: AccountAddress,
        for_sale_params: &ForSale,
    ) -> ContractResult<()> {
        let token_info = TokenInfo {
            token: for_sale_params.token.clone(),
            owner,
            for_sale: for_sale_params.for_sale,
        };
        self.token_details.push(token_info);

        Ok(())
    }
}
