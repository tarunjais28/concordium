use super::*;

// Functions for creating and updating the contract state.
impl State {
    /// Creates a new state with account info.
    pub fn init(address: AccountAddress) -> Self {
        Self { account: address }
    }
}
