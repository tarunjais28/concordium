use super::*;

// Functions for creating and updating the contract state.
impl<S: HasStateApi> State<S> {
    /// Creates a new state with Send.
    pub fn empty() -> Self {
        State {
            phantom_data: PhantomData,
        }
    }
}
