use super::*;

// Functions for creating and updating the contract state.
impl State {
    /// Creates a new state with no listings.
    pub fn new(storage_address: ContractAddress) -> Self {
        State {
            storage_address,
            function_states: None,
            leaf_contract_address: None,
        }
    }
}

impl Shares {
    pub fn adjust_owner_share(&mut self) {
        self.owner -= self.creator + self.minter + self.bictory
    }
}

impl Default for NFTDetails {
    fn default() -> Self {
        let default_acc_addr = AccountAddress([0; 32]);
        Self {
            token_id: TokenIdVec([0; 32].to_vec()),
            owner: default_acc_addr,
            creator: default_acc_addr,
            creator_royalty: 0,
            minter: default_acc_addr,
            minter_royalty: 0,
            price: Amount::zero(),
            for_sale: false,
        }
    }
}
