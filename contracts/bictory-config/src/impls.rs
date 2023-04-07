use super::*;

// Functions for creating and updating the contract state.
impl<S: HasStateApi> State<S> {
    /// Creates a new state with account info.
    pub fn init(address: AccountAddress) -> Self {
        Self {
            account: address,
            phantom_data: PhantomData,
        }
    }

    /// Update state with new address.
    pub fn update_address(&mut self, address: AccountAddress) {
        self.account = address;
    }
}
